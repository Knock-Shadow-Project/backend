use axum::{
    Form, Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::auth::{hash_password, jwt_secret};
use crate::state::AppState;

const RESEND_COOLDOWN_SECS: u64 = 60;
const RATE_LIMIT_PREFIX: &str = "rate_limit:resend:";
const RESET_COOLDOWN_SECS: u64 = 60;
const RESET_RATE_LIMIT_PREFIX: &str = "rate_limit:password_reset:";

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ResendRequest {
    pub email: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ResendResponse {
    pub status: String,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfirmationClaims {
    sub: String,
    purpose: String,
    exp: usize,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/resend-confirmation", post(resend_confirmation))
        .route("/confirm-email", get(confirm_email))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", get(reset_password_form).post(reset_password_submit))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ConfirmEmailQuery {
    pub token: String,
}

#[utoipa::path(get, path = "/confirm-email",
    params(ConfirmEmailQuery),
    responses(
        (status = 200, description = "Email confirmed (HTML page)", content_type = "text/html"),
        (status = 400, description = "Invalid or expired token"),
        (status = 404, description = "User not found"),
    ),
    tag = "Email"
)]
pub(crate) async fn confirm_email(
    State(state): State<AppState>,
    Query(q): Query<ConfirmEmailQuery>,
) -> (StatusCode, Html<String>) {
    let token = q.token.trim();
    if token.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Html(html_error("El enlace está vacío o mal formado.")),
        );
    }

    let claims = match decode::<ConfirmationClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data.claims,
        Err(e) => {
            tracing::debug!("confirm-email: token inválido: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Html(html_error(
                    "El enlace es inválido o ha caducado. Pide uno nuevo desde la app.",
                )),
            );
        }
    };

    if claims.purpose != "email_confirm" {
        tracing::warn!(
            "confirm-email: token con purpose incorrecto ({})",
            claims.purpose
        );
        return (
            StatusCode::BAD_REQUEST,
            Html(html_error("El enlace no es válido para confirmar correo.")),
        );
    }

    let email = claims.sub.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return (
            StatusCode::BAD_REQUEST,
            Html(html_error("El enlace está malformado (correo vacío).")),
        );
    }

    let updated =
        sqlx::query("UPDATE usuario SET confirmado = TRUE WHERE correo = $1 RETURNING id_usuario")
            .bind(&email)
            .fetch_optional(&state.pool)
            .await;

    match updated {
        Ok(Some(_)) => {
            tracing::info!("confirm-email: cuenta confirmada para {}", email);
            (StatusCode::OK, Html(html_success(&email)))
        }
        Ok(None) => {
            tracing::warn!("confirm-email: usuario no encontrado para {}", email);
            (
                StatusCode::NOT_FOUND,
                Html(html_error(
                    "No encontramos esa cuenta. Quizás se eliminó después de pedir la confirmación.",
                )),
            )
        }
        Err(e) => {
            tracing::error!("confirm-email: error de BD al confirmar {}: {}", email, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(html_error(
                    "Tuvimos un problema interno. Vuelve a intentarlo en unos minutos.",
                )),
            )
        }
    }
}

fn html_success(email: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>Cuenta confirmada — KnockShadow</title>
</head>
<body style="margin:0;padding:0;background-color:#131315;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;color:#e5e1e4;min-height:100vh;display:flex;align-items:center;justify-content:center;">
  <main style="max-width:520px;width:90%;background-color:#1b1b1d;border:1px solid #27272a;border-radius:16px;padding:36px;text-align:center;">
    <div style="display:inline-flex;align-items:center;justify-content:center;width:72px;height:72px;border-radius:36px;background-color:rgba(108,215,216,0.1);border:1px solid rgba(108,215,216,0.3);margin-bottom:18px;">
      <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#6cd7d8" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
    </div>
    <p style="margin:0 0 6px;color:#ff525c;font-size:10px;font-weight:bold;letter-spacing:3px;text-transform:uppercase;">CUENTA VERIFICADA</p>
    <h1 style="margin:0;color:#fff;font-size:26px;font-weight:900;letter-spacing:-0.5px;">¡Listo, atleta!</h1>
    <p style="margin:14px 0 0;color:#a1a1aa;font-size:14px;line-height:22px;">
      Hemos confirmado <strong style="color:#ffffff;">{email}</strong>. Ya puedes abrir la app y empezar a registrar telemetría.
    </p>
    <a href="knockshadowfront://login" style="display:inline-block;margin-top:24px;background-color:#ff525c;color:#410008;font-weight:900;letter-spacing:2px;text-decoration:none;padding:14px 28px;border-radius:8px;font-size:13px;">ABRIR APP</a>
    <p style="margin:18px 0 0;color:#52525b;font-size:11px;">
      ¿No se abre? <a href="knockshadowfront://login" style="color:#6cd7d8;text-decoration:underline;">Pulsa aquí</a> o entra desde tu móvil.
    </p>
    <p style="margin:28px 0 0;padding-top:18px;border-top:1px solid #27272a;color:#52525b;font-size:10px;letter-spacing:1.5px;text-transform:uppercase;">
      KNOCKSHADOW · TELEMETRÍA BIOMÉTRICA
    </p>
  </main>
</body>
</html>"##
    )
}

fn html_error(mensaje: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>Error de confirmación — KnockShadow</title>
</head>
<body style="margin:0;padding:0;background-color:#131315;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;color:#e5e1e4;min-height:100vh;display:flex;align-items:center;justify-content:center;">
  <main style="max-width:520px;width:90%;background-color:#1b1b1d;border:1px solid #27272a;border-radius:16px;padding:36px;text-align:center;">
    <div style="display:inline-flex;align-items:center;justify-content:center;width:72px;height:72px;border-radius:36px;background-color:rgba(255,82,92,0.1);border:1px solid rgba(255,82,92,0.3);margin-bottom:18px;">
      <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#ff525c" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
    </div>
    <p style="margin:0 0 6px;color:#ff525c;font-size:10px;font-weight:bold;letter-spacing:3px;text-transform:uppercase;">ENLACE NO VÁLIDO</p>
    <h1 style="margin:0;color:#fff;font-size:24px;font-weight:900;letter-spacing:-0.5px;">No pudimos confirmar tu cuenta</h1>
    <p style="margin:14px 0 0;color:#a1a1aa;font-size:14px;line-height:22px;">
      {mensaje}
    </p>
    <a href="knockshadowfront://confirmEmail" style="display:inline-block;margin-top:24px;background-color:transparent;color:#ff525c;border:1px solid #ff525c;font-weight:900;letter-spacing:2px;text-decoration:none;padding:13px 28px;border-radius:8px;font-size:13px;">PEDIR UN NUEVO ENLACE</a>
    <p style="margin:28px 0 0;padding-top:18px;border-top:1px solid #27272a;color:#52525b;font-size:10px;letter-spacing:1.5px;text-transform:uppercase;">
      KNOCKSHADOW · TELEMETRÍA BIOMÉTRICA
    </p>
  </main>
</body>
</html>"##
    )
}

#[utoipa::path(post, path = "/resend-confirmation",
    request_body = ResendRequest,
    responses(
        (status = 200, body = ResendResponse),
        (status = 400, description = "Invalid email"),
        (status = 429, description = "Too many requests"),
    ),
    tag = "Email"
)]
pub(crate) async fn resend_confirmation(
    State(state): State<AppState>,
    Json(payload): Json<ResendRequest>,
) -> Result<Json<ResendResponse>, StatusCode> {
    let email = payload.email.trim().to_lowercase();

    if email.is_empty() || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let rl_key = format!("{}{}", RATE_LIMIT_PREFIX, email);
    if !state
        .rate_limiter
        .try_acquire(&rl_key, RESEND_COOLDOWN_SECS)
        .await
    {
        tracing::info!(
            "[EMAIL] Rate limit: {} pidió reenvío demasiado pronto",
            email
        );
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM usuario WHERE correo = $1)")
        .bind(&email)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("DB error in resend-confirmation: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if exists {
        let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
        let claims = ConfirmationClaims {
            sub: email.clone(),
            purpose: "email_confirm".to_string(),
            exp,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret().as_bytes()),
        )
        .map_err(|e| {
            tracing::error!("JWT encode error in resend-confirmation: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        match state.email_service {
            Some(svc) => {
                let email_para_log = email.clone();
                tokio::spawn(async move {
                    match svc.send_confirmation(&email_para_log, &token).await {
                        Ok(id) => tracing::info!(
                            "[EMAIL] Confirmación enviada a {} (resend_id={})",
                            email_para_log,
                            id
                        ),
                        Err(e) => tracing::error!(
                            "[EMAIL] Falló envío Resend a {}: {}",
                            email_para_log,
                            e
                        ),
                    }
                });
            }
            None => {
                tracing::info!(
                    "[EMAIL STUB] Reenviar confirmación a {} — token={}",
                    email,
                    token
                );
            }
        }
    } else {
        tracing::info!(
            "[EMAIL] Reenvío solicitado para email no registrado: {}",
            email
        );
    }

    Ok(Json(ResendResponse {
        status: "queued".to_string(),
        message: "Si el correo está registrado, recibirás un nuevo email en breve.".to_string(),
    }))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[utoipa::path(post, path = "/forgot-password",
    request_body = ForgotPasswordRequest,
    responses(
        (status = 200, body = ResendResponse),
        (status = 400, description = "Invalid email"),
        (status = 429, description = "Too many requests"),
    ),
    tag = "Email"
)]
pub(crate) async fn forgot_password(
    State(state): State<AppState>,
    Json(payload): Json<ForgotPasswordRequest>,
) -> Result<Json<ResendResponse>, StatusCode> {
    let email = payload.email.trim().to_lowercase();

    if email.is_empty() || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }

    let rl_key = format!("{}{}", RESET_RATE_LIMIT_PREFIX, email);
    if !state
        .rate_limiter
        .try_acquire(&rl_key, RESET_COOLDOWN_SECS)
        .await
    {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM usuario WHERE correo = $1)")
        .bind(&email)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("DB error in forgot-password: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if exists {
        let exp = (Utc::now() + Duration::hours(1)).timestamp() as usize;
        let claims = ConfirmationClaims {
            sub: email.clone(),
            purpose: "password_reset".to_string(),
            exp,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(jwt_secret().as_bytes()),
        )
        .map_err(|e| {
            tracing::error!("JWT encode error in forgot-password: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        match state.email_service {
            Some(svc) => {
                let email_log = email.clone();
                tokio::spawn(async move {
                    match svc.send_password_reset(&email_log, &token).await {
                        Ok(id) => tracing::info!(
                            "[EMAIL] Reset password enviado a {} (resend_id={})",
                            email_log, id
                        ),
                        Err(e) => tracing::error!(
                            "[EMAIL] Falló envío reset a {}: {}",
                            email_log, e
                        ),
                    }
                });
            }
            None => {
                tracing::info!(
                    "[EMAIL STUB] Reset password para {} — token={}",
                    email, token
                );
            }
        }
    } else {
        tracing::info!(
            "[EMAIL] Reset solicitado para email no registrado: {}",
            email
        );
    }

    Ok(Json(ResendResponse {
        status: "queued".to_string(),
        message: "Si el correo está registrado, recibirás un email para restablecer tu contraseña."
            .to_string(),
    }))
}

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct ResetPasswordQuery {
    pub token: String,
}

#[utoipa::path(get, path = "/reset-password",
    params(ResetPasswordQuery),
    responses(
        (status = 200, description = "Password reset form (HTML page)", content_type = "text/html"),
        (status = 400, description = "Invalid or expired token"),
    ),
    tag = "Email"
)]
pub(crate) async fn reset_password_form(
    Query(q): Query<ResetPasswordQuery>,
) -> (StatusCode, Html<String>) {
    let token = q.token.trim();
    if token.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Html(html_reset_error("El enlace está vacío o mal formado.")),
        );
    }

    let claims = match decode::<ConfirmationClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data.claims,
        Err(e) => {
            tracing::debug!("reset-password: token inválido: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Html(html_reset_error(
                    "El enlace es inválido o ha caducado. Solicita uno nuevo desde la app.",
                )),
            );
        }
    };

    if claims.purpose != "password_reset" {
        return (
            StatusCode::BAD_REQUEST,
            Html(html_reset_error("El enlace no es válido para restablecer contraseña.")),
        );
    }

    (StatusCode::OK, Html(html_reset_form(token)))
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ResetPasswordForm {
    pub token: String,
    pub password: String,
}

#[utoipa::path(post, path = "/reset-password",
    request_body = ResetPasswordForm,
    responses(
        (status = 200, description = "Password reset success (HTML page)", content_type = "text/html"),
        (status = 400, description = "Invalid token or weak password"),
        (status = 404, description = "User not found"),
    ),
    tag = "Email"
)]
pub(crate) async fn reset_password_submit(
    State(state): State<AppState>,
    Form(form): Form<ResetPasswordForm>,
) -> (StatusCode, Html<String>) {
    let token = form.token.trim();
    if token.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Html(html_reset_error("Token vacío.")),
        );
    }

    let claims = match decode::<ConfirmationClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data.claims,
        Err(e) => {
            tracing::debug!("reset-password-submit: token inválido: {}", e);
            return (
                StatusCode::BAD_REQUEST,
                Html(html_reset_error(
                    "El enlace es inválido o ha caducado. Solicita uno nuevo desde la app.",
                )),
            );
        }
    };

    if claims.purpose != "password_reset" {
        return (
            StatusCode::BAD_REQUEST,
            Html(html_reset_error("El enlace no es válido para restablecer contraseña.")),
        );
    }

    let password = form.password.trim();
    if password.len() < 6 {
        return (
            StatusCode::BAD_REQUEST,
            Html(html_reset_error("La contraseña debe tener al menos 6 caracteres.")),
        );
    }

    let hashed = match hash_password(password) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("reset-password: bcrypt error: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(html_reset_error("Error interno. Inténtalo de nuevo.")),
            );
        }
    };

    let email = claims.sub.trim().to_lowercase();
    let updated = sqlx::query(
        "UPDATE usuario SET contrasena = $1 WHERE correo = $2 RETURNING id_usuario",
    )
    .bind(&hashed)
    .bind(&email)
    .fetch_optional(&state.pool)
    .await;

    match updated {
        Ok(Some(_)) => {
            tracing::info!("reset-password: contraseña actualizada para {}", email);
            (StatusCode::OK, Html(html_reset_success(&email)))
        }
        Ok(None) => {
            tracing::warn!("reset-password: usuario no encontrado para {}", email);
            (
                StatusCode::NOT_FOUND,
                Html(html_reset_error("No encontramos esa cuenta.")),
            )
        }
        Err(e) => {
            tracing::error!("reset-password: DB error para {}: {}", email, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Html(html_reset_error("Error interno. Inténtalo de nuevo en unos minutos.")),
            )
        }
    }
}

fn html_reset_form(token: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>Nueva contraseña — KnockShadow</title>
</head>
<body style="margin:0;padding:0;background-color:#131315;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;color:#e5e1e4;min-height:100vh;display:flex;align-items:center;justify-content:center;">
  <main style="max-width:520px;width:90%;background-color:#1b1b1d;border:1px solid #27272a;border-radius:16px;padding:36px;text-align:center;">
    <div style="display:inline-flex;align-items:center;justify-content:center;width:72px;height:72px;border-radius:36px;background-color:rgba(108,215,216,0.1);border:1px solid rgba(108,215,216,0.3);margin-bottom:18px;">
      <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#6cd7d8" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg>
    </div>
    <p style="margin:0 0 6px;color:#ff525c;font-size:10px;font-weight:bold;letter-spacing:3px;text-transform:uppercase;">NUEVA CONTRASEÑA</p>
    <h1 style="margin:0 0 8px;color:#fff;font-size:24px;font-weight:900;letter-spacing:-0.5px;">Crea tu nueva contraseña</h1>
    <p style="margin:0 0 24px;color:#a1a1aa;font-size:14px;line-height:22px;">Ingresa una contraseña segura de al menos 6 caracteres.</p>
    <form method="POST" action="/reset-password" style="text-align:left;">
      <input type="hidden" name="token" value="{token}" />
      <label style="display:block;color:#71717a;font-size:10px;font-weight:bold;letter-spacing:2px;text-transform:uppercase;margin-bottom:6px;">CONTRASEÑA</label>
      <input type="password" name="password" required minlength="6" placeholder="Mínimo 6 caracteres" style="width:100%;box-sizing:border-box;padding:14px 16px;background-color:#131315;border:1px solid #2d2d30;border-radius:8px;color:#fff;font-size:14px;font-weight:bold;letter-spacing:1px;margin-bottom:20px;outline:none;" onfocus="this.style.borderColor='#ff525c'" onblur="this.style.borderColor='#2d2d30'" />
      <button type="submit" style="width:100%;padding:14px;background-color:#ff525c;color:#410008;font-weight:900;letter-spacing:2px;border:none;border-radius:8px;font-size:14px;cursor:pointer;">GUARDAR CONTRASEÑA</button>
    </form>
    <p style="margin:28px 0 0;padding-top:18px;border-top:1px solid #27272a;color:#52525b;font-size:10px;letter-spacing:1.5px;text-transform:uppercase;">
      KNOCKSHADOW · TELEMETRÍA BIOMÉTRICA
    </p>
  </main>
</body>
</html>"##
    )
}

fn html_reset_success(email: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>Contraseña actualizada — KnockShadow</title>
</head>
<body style="margin:0;padding:0;background-color:#131315;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;color:#e5e1e4;min-height:100vh;display:flex;align-items:center;justify-content:center;">
  <main style="max-width:520px;width:90%;background-color:#1b1b1d;border:1px solid #27272a;border-radius:16px;padding:36px;text-align:center;">
    <div style="display:inline-flex;align-items:center;justify-content:center;width:72px;height:72px;border-radius:36px;background-color:rgba(108,215,216,0.1);border:1px solid rgba(108,215,216,0.3);margin-bottom:18px;">
      <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#6cd7d8" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"></polyline></svg>
    </div>
    <p style="margin:0 0 6px;color:#ff525c;font-size:10px;font-weight:bold;letter-spacing:3px;text-transform:uppercase;">CONTRASEÑA ACTUALIZADA</p>
    <h1 style="margin:0;color:#fff;font-size:26px;font-weight:900;letter-spacing:-0.5px;">¡Listo, atleta!</h1>
    <p style="margin:14px 0 0;color:#a1a1aa;font-size:14px;line-height:22px;">
      La contraseña de <strong style="color:#ffffff;">{email}</strong> ha sido actualizada. Ya puedes iniciar sesión con tu nueva contraseña.
    </p>
    <a href="knockshadowfront://login" style="display:inline-block;margin-top:24px;background-color:#ff525c;color:#410008;font-weight:900;letter-spacing:2px;text-decoration:none;padding:14px 28px;border-radius:8px;font-size:13px;">ABRIR APP</a>
    <p style="margin:18px 0 0;color:#52525b;font-size:11px;">
      ¿No se abre? <a href="knockshadowfront://login" style="color:#6cd7d8;text-decoration:underline;">Pulsa aquí</a> o entra desde tu móvil.
    </p>
    <p style="margin:28px 0 0;padding-top:18px;border-top:1px solid #27272a;color:#52525b;font-size:10px;letter-spacing:1.5px;text-transform:uppercase;">
      KNOCKSHADOW · TELEMETRÍA BIOMÉTRICA
    </p>
  </main>
</body>
</html>"##
    )
}

fn html_reset_error(mensaje: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>Error — KnockShadow</title>
</head>
<body style="margin:0;padding:0;background-color:#131315;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;color:#e5e1e4;min-height:100vh;display:flex;align-items:center;justify-content:center;">
  <main style="max-width:520px;width:90%;background-color:#1b1b1d;border:1px solid #27272a;border-radius:16px;padding:36px;text-align:center;">
    <div style="display:inline-flex;align-items:center;justify-content:center;width:72px;height:72px;border-radius:36px;background-color:rgba(255,82,92,0.1);border:1px solid rgba(255,82,92,0.3);margin-bottom:18px;">
      <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#ff525c" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
    </div>
    <p style="margin:0 0 6px;color:#ff525c;font-size:10px;font-weight:bold;letter-spacing:3px;text-transform:uppercase;">ERROR</p>
    <h1 style="margin:0;color:#fff;font-size:24px;font-weight:900;letter-spacing:-0.5px;">No se pudo restablecer</h1>
    <p style="margin:14px 0 0;color:#a1a1aa;font-size:14px;line-height:22px;">
      {mensaje}
    </p>
    <a href="knockshadowfront://forgotPassword" style="display:inline-block;margin-top:24px;background-color:transparent;color:#ff525c;border:1px solid #ff525c;font-weight:900;letter-spacing:2px;text-decoration:none;padding:13px 28px;border-radius:8px;font-size:13px;">SOLICITAR NUEVO ENLACE</a>
    <p style="margin:28px 0 0;padding-top:18px;border-top:1px solid #27272a;color:#52525b;font-size:10px;letter-spacing:1.5px;text-transform:uppercase;">
      KNOCKSHADOW · TELEMETRÍA BIOMÉTRICA
    </p>
  </main>
</body>
</html>"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_claims_serialize_with_purpose() {
        let claims = ConfirmationClaims {
            sub: "x@y.com".to_string(),
            purpose: "email_confirm".to_string(),
            exp: 0,
        };
        let json = serde_json::to_string(&claims).unwrap();
        assert!(json.contains("\"purpose\":\"email_confirm\""));
        assert!(json.contains("\"sub\":\"x@y.com\""));
    }

    #[test]
    fn html_success_embeds_email_and_deep_link() {
        let html = html_success("vic@example.com");
        assert!(html.contains("vic@example.com"));
        assert!(html.contains("knockshadowfront://login"));
        assert!(html.contains("#ff525c"));
    }

    #[test]
    fn html_error_embeds_message_and_recovery_link() {
        let html = html_error("Mensaje custom de error.");
        assert!(html.contains("Mensaje custom de error."));
        assert!(html.contains("knockshadowfront://confirmEmail"));
    }
}
