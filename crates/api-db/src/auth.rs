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
use std::sync::OnceLock;

use crate::models::{CreateUsuario, LoginRequest, LoginResponse, TokenClaims, Usuario};
use crate::state::AppState;

const BEARER: &str = "Bearer ";

/// Minimum length required for a JWT secret (bits/8 for HS256).
const MIN_SECRET_LEN: usize = 32;

/// Holds the JWT signing secret loaded once at startup.
///
/// We deliberately avoid environment-variable fallbacks: an unset or weak
/// secret in production silently breaks token verification across restarts
/// and weakens forgery resistance, so we fail fast at boot instead.
static JWT_SECRET: OnceLock<String> = OnceLock::new();

/// Loads `JWT_SECRET` from the environment and stores it in the global cell.
///
/// Must be called exactly once at startup, before any handler runs.
/// Returns an error if the variable is unset, empty, or shorter than the
/// minimum recommended length.
pub fn init_jwt_secret() -> Result<(), String> {
    let secret = std::env::var("JWT_SECRET").map_err(|_| {
        "JWT_SECRET environment variable is required but not set. \
         Generate one with: openssl rand -hex 32"
            .to_string()
    })?;
    let trimmed_len = secret.trim().len();
    if trimmed_len == 0 {
        return Err("JWT_SECRET must not be empty".to_string());
    }
    if trimmed_len < MIN_SECRET_LEN {
        return Err(format!(
            "JWT_SECRET must be at least {} characters (got {})",
            MIN_SECRET_LEN, trimmed_len
        ));
    }
    JWT_SECRET
        .set(secret)
        .map_err(|_| "JWT_SECRET already initialized".to_string())?;
    tracing::info!("JWT secret loaded ({} bytes)", trimmed_len);
    Ok(())
}

/// Returns the JWT signing secret loaded via [`init_jwt_secret`].
///
/// Panics if called before initialization — that would be a programmer error.
pub fn jwt_secret() -> &'static str {
    JWT_SECRET
        .get()
        .expect("jwt_secret() called before init_jwt_secret() — this is a bug")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize tests touching the JWT_SECRET OnceLock and process env.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn init_jwt_secret_rejects_unset() {
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe { std::env::remove_var("JWT_SECRET") };
        let err = init_jwt_secret().unwrap_err();
        assert!(err.contains("JWT_SECRET"));
    }

    #[test]
    fn init_jwt_secret_rejects_too_short() {
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe { std::env::set_var("JWT_SECRET", "short") };
        let err = init_jwt_secret().unwrap_err();
        assert!(err.contains("at least"));
        unsafe { std::env::remove_var("JWT_SECRET") };
    }

    #[test]
    fn init_jwt_secret_rejects_empty() {
        let _guard = TEST_LOCK.lock().unwrap();
        unsafe { std::env::set_var("JWT_SECRET", "   ") };
        let err = init_jwt_secret().unwrap_err();
        assert!(err.contains("empty"));
        unsafe { std::env::remove_var("JWT_SECRET") };
    }
}
