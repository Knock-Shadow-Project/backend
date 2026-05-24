use axum::Router;
use axum::middleware;

mod email;
mod entrenamiento;
mod golpe;
mod historial;
mod rutina;
mod usuario;
mod ws;

use crate::auth::{auth_middleware, login_handler, register_handler};
use crate::state::AppState;

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok", "service": "api-db" }))
}

pub fn router() -> Router<AppState> {
    // Public routes (no auth required).
    //
    // `/resend-confirmation` vive aquí (no en `protected`) porque el atleta
    // típicamente lo pulsará desde la pantalla `confirmEmail.tsx` antes de
    // haber iniciado sesión por primera vez, así que no tiene JWT.
    let public = Router::new()
        .route("/health", axum::routing::get(health))
        .route("/login", axum::routing::post(login_handler))
        .route("/register", axum::routing::post(register_handler))
        .merge(email::routes());

    // Protected routes (Bearer token required)
    let protected = Router::new()
        .merge(usuario::routes())
        .merge(entrenamiento::routes())
        .merge(golpe::routes())
        .merge(historial::routes())
        .merge(rutina::routes())
        .route("/ws", axum::routing::get(ws::ws_handler))
        .layer(middleware::from_fn(auth_middleware));

    public.merge(protected)
}
