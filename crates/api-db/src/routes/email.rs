//! Endpoints relacionados con confirmación de correo del atleta.
//!
//! Flujo end-to-end:
//!
//!   1. `POST /resend-confirmation` (público) valida el email, comprueba
//!      rate-limit (1 req/min/email) y, si el usuario existe en `usuario`,
//!      genera un JWT con `purpose=email_confirm` (TTL 24h, mismo
//!      `JWT_SECRET` que los tokens de sesión).
//!   2. Si hay `EmailService` en `AppState` (es decir, `RESEND_API_KEY`),
//!      el handler **envía el email real** vía Resend en `tokio::spawn`
//!      para no bloquear la respuesta HTTP. Sin `EmailService` cae a modo
//!      **stub** (loggea el enlace).
//!   3. La respuesta es siempre `200` con un mensaje genérico aunque el
//!      email no exista, para no permitir enumeración de cuentas.
//!   4. `GET /confirm-email?token=...` (público) decodifica el JWT,
//!      valida `purpose=email_confirm`, marca `usuario.confirmado=TRUE`
//!      y devuelve una **página HTML** (no JSON) porque el usuario llega
//!      desde un cliente de correo abierto en un navegador.

use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};

use crate::auth::jwt_secret;
use crate::state::AppState;

/// Tiempo mínimo entre dos reenvíos para el mismo email. Se reusa como
/// TTL en Redis (con `SET ... EX <secs>`) cuando ese backend está activo.
const RESEND_COOLDOWN_SECS: u64 = 60;

/// Prefijo de la clave en Redis. Lo mantenemos namespaced para poder
/// añadir otras categorías de rate-limit en el futuro sin colisionar.
const RATE_LIMIT_PREFIX: &str = "rate_limit:resend:";

#[derive(Debug, Deserialize)]
pub struct ResendRequest {
    pub email: String,
}

#[derive(Debug, Serialize)]
pub struct ResendResponse {
    pub status: String,
    pub message: String,
}

/// Claims del JWT de confirmación. Reutilizamos el mismo secreto y algoritmo
/// que el JWT de sesión, pero con un `purpose` distinto para que el
/// middleware de auth no acepte un token de confirmación como token de
/// sesión y viceversa.
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
}

#[derive(Debug, Deserialize)]
pub struct ConfirmEmailQuery {
    pub token: String,
}

/// Handler de la URL pública que llega en el email. Devuelve HTML porque
/// el atleta abre el enlace desde su gestor de correo en un navegador,
/// no desde la app.
///
/// Estados posibles:
///   - 200 OK: cuenta confirmada (o ya estaba confirmada, idempotente).
///   - 400 Bad Request: token vacío, mal formado, expirado o con purpose
///     incorrecto. La pantalla HTML explica al usuario qué hacer.
///   - 404 Not Found: el email del token ya no existe en la base
///     (cuenta borrada entre que se generó el token y se hizo clic).
async fn confirm_email(
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

    // Validación del JWT. Cualquier fallo (firma, exp, malformed) cae
    // a la misma pantalla genérica para no dar pistas sobre la causa
    // exacta a un atacante que esté probando tokens al azar.
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

    // El token de sesión y el de confirmación se firman con el mismo
    // secreto pero con `purpose` distinto. Validar el purpose evita que
    // un token de sesión robado se use como confirmación y viceversa.
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

    // UPDATE idempotente: si ya estaba confirmado devolvemos el mismo
    // estado de éxito en vez de un error, así un usuario que reabre el
    // enlace por error no se asusta. `rows_affected` distingue entre
    // "se marcó ahora" y "ya estaba" sólo para el log.
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

/// Página HTML servida cuando la confirmación va bien. Mantiene el HUD
/// rojo/oscuro del producto y ofrece un deep-link a la app (`knockshadowfront://login`)
/// y un enlace de fallback web al login.
fn html_success(email: &str) -> String {
    // Nota sobre los delimitadores: usamos `r##"..."##` (dos almohadillas)
    // porque dentro del HTML aparece la secuencia `"#` (p.ej.
    // `stroke="#6cd7d8"`), que con `r#"..."#` cerraría la raw string a
    // mitad. Subir el nivel a `##` da margen.
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

/// Página HTML servida cuando la confirmación falla. Mensajes humanos,
/// sin exponer detalles técnicos al usuario final.
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

async fn resend_confirmation(
    State(state): State<AppState>,
    Json(payload): Json<ResendRequest>,
) -> Result<Json<ResendResponse>, StatusCode> {
    let email = payload.email.trim().to_lowercase();

    if email.is_empty() || !email.contains('@') {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Rate-limit: 1 reenvío/min por email. El backend (in-memory o Redis)
    // se elige al arranque vía `RateLimiter::from_url(REDIS_URL)`; aquí
    // sólo invocamos la interfaz común. Fail-open: si el backend remoto
    // está KO, `try_acquire` devuelve `true` (permitimos) y loggea.
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

        // Envío real (Resend) o stub (log) según haya `EmailService` en
        // el estado. En ambos casos no bloqueamos el handler: la respuesta
        // HTTP devuelve `200` enseguida y el envío se completa en spawn.
        // Si Resend falla, lo veremos en `tracing::error!` pero no se lo
        // contamos al cliente (mantiene la respuesta genérica).
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
        // Importante: logueamos para depurar, pero respondemos lo mismo
        // que si el usuario existiera. Esto cierra la puerta a un atacante
        // que use el endpoint para descubrir si un email está registrado.
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
        // Deep-link al scheme registrado en `app.json` de la app móvil.
        assert!(html.contains("knockshadowfront://login"));
        // Color HUD del producto presente (botón principal).
        assert!(html.contains("#ff525c"));
    }

    #[test]
    fn html_error_embeds_message_and_recovery_link() {
        let html = html_error("Mensaje custom de error.");
        assert!(html.contains("Mensaje custom de error."));
        // El CTA de error envía al usuario a pedir un nuevo enlace.
        assert!(html.contains("knockshadowfront://confirmEmail"));
    }
}
