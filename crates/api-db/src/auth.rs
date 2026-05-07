use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::Response,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};

use crate::models::{CreateUsuario, LoginRequest, LoginResponse, TokenClaims, Usuario};
use crate::state::AppState;

const BEARER: &str = "Bearer ";

pub fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!("JWT_SECRET not set, using insecure default");
        "knockshadow_default_secret_change_me".to_string()
    })
}

pub fn create_token(user_id: i32, email: String) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = TokenClaims {
        sub: user_id,
        email,
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
}

pub fn decode_token(token: &str) -> Result<TokenClaims, jsonwebtoken::errors::Error> {
    decode::<TokenClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, bcrypt::BcryptError> {
    bcrypt::verify(password, hash)
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let row = sqlx::query_as::<_, (i32, String, String)>(
        "SELECT id_usuario, nombre, contrasena FROM usuario WHERE correo = $1",
    )
    .bind(&payload.email)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::UNAUTHORIZED,
        _ => {
            tracing::error!("DB error during login: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let (user_id, first_name, stored_password) = row;

    if !verify_password(&payload.password, &stored_password).map_err(|e| {
        tracing::error!("Password verify error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })? {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = create_token(user_id, payload.email.clone()).map_err(|e| {
        tracing::error!("JWT encode error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(LoginResponse {
        token,
        user_id,
        first_name,
        email: payload.email,
    }))
}

pub async fn auth_middleware(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with(BEARER) => &header[BEARER.len()..],
        _ => {
            tracing::debug!("Missing or malformed Authorization header");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    let claims = decode_token(token).map_err(|e| {
        tracing::debug!("Invalid token: {}", e);
        StatusCode::UNAUTHORIZED
    })?;

    // Inject claims into request extensions so handlers can use them
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateUsuario>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let hashed_password = hash_password(&payload.password).map_err(|e| {
        tracing::error!("Password hash error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let usuario = sqlx::query_as::<_, Usuario>(
        "INSERT INTO usuario (nombre, apellido, correo, contrasena, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel",
    )
    .bind(&payload.first_name)
    .bind(&payload.last_name)
    .bind(&payload.email)
    .bind(&hashed_password)
    .bind(&payload.phone)
    .bind(payload.age)
    .bind(payload.weight)
    .bind(payload.height)
    .bind(&payload.country)
    .bind(&payload.city)
    .bind(&payload.address)
    .bind(&payload.laterality)
    .bind(&payload.level)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create usuario: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let token = create_token(usuario.user_id, usuario.email.clone()).map_err(|e| {
        tracing::error!("JWT encode error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(LoginResponse {
        token,
        user_id: usuario.user_id,
        first_name: usuario.first_name,
        email: usuario.email,
    }))
}
