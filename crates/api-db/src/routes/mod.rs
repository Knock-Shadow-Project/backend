use axum::Router;
use axum::middleware;

mod entrenamiento;
mod golpe;
mod historial;
mod usuario;
mod ws;

use crate::auth::{auth_middleware, login_handler, register_handler};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    // Public routes (no auth required)
    let public = Router::new()
        .route("/login", axum::routing::post(login_handler))
        .route("/register", axum::routing::post(register_handler));

    // Protected routes (Bearer token required)
    let protected = Router::new()
        .merge(usuario::routes())
        .merge(entrenamiento::routes())
        .merge(golpe::routes())
        .merge(historial::routes())
        .route("/ws", axum::routing::get(ws::ws_handler))
        .layer(middleware::from_fn(auth_middleware));

    public.merge(protected)
}
