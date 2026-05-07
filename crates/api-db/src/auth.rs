use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::Response,
    Json,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};


use crate::models::{CreateUsuario, LoginRequest, LoginResponse, TokenClaims, Usuario};
use crate::state::AppState;

const BEARER: &str = "Bearer ";

pub fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| {
        tracing::warn!("JWT_SECRET not set, using insecure default");
        "knockshadow_default_secret_change_me".to_string()
    })
}

pub fn create_token(id_usuario: i32, email: String) -> Result<String, jsonwebtoken::errors::Error> {
    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = TokenClaims {
        sub: id_usuario,
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

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let row = sqlx::query_as::<_, (i32, String, String)>(
        "SELECT id_usuario, nombre, contrasena FROM usuario WHERE correo = $1",
    )
    .bind(&payload.correo)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::UNAUTHORIZED,
        _ => {
            tracing::error!("DB error during login: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;

    let (id_usuario, nombre, stored_password) = row;

    if stored_password != payload.contrasena {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = create_token(id_usuario, payload.correo.clone()).map_err(|e| {
        tracing::error!("JWT encode error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(LoginResponse {
        token,
        id_usuario,
        nombre,
        correo: payload.correo,
    }))
}

pub async fn auth_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
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

/// Returns the claims if present, otherwise 500 (should never happen when middleware is applied).
pub fn require_auth(req: &Request<Body>) -> Result<&TokenClaims, StatusCode> {
    req.extensions()
        .get::<TokenClaims>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateUsuario>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let usuario = sqlx::query_as::<_, Usuario>(
        "INSERT INTO usuario (nombre, apellido, correo, contrasena, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel",
    )
    .bind(&payload.nombre)
    .bind(&payload.apellido)
    .bind(&payload.correo)
    .bind(&payload.contrasena)
    .bind(&payload.telefono)
    .bind(payload.edad)
    .bind(payload.peso)
    .bind(payload.estatura)
    .bind(&payload.pais)
    .bind(&payload.ciudad)
    .bind(&payload.direccion)
    .bind(&payload.lateralidad)
    .bind(&payload.nivel)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create usuario: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let token = create_token(usuario.id_usuario, usuario.correo.clone()).map_err(|e| {
        tracing::error!("JWT encode error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(LoginResponse {
        token,
        id_usuario: usuario.id_usuario,
        nombre: usuario.nombre,
        correo: usuario.correo,
    }))
}
