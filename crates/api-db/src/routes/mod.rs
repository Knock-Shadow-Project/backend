use axum::Router;
use axum::middleware;
use utoipa::OpenApi;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};

mod email;
mod entrenamiento;
mod golpe;
mod historial;
mod resultado_fuerza;
mod rutina;
mod usuario;
mod ws;

use crate::auth::{auth_middleware, login_handler, register_handler};
use crate::models::*;
use crate::state::AppState;

#[utoipa::path(get, path = "/health",
    responses((status = 200, description = "Service is healthy")),
    tag = "Health"
)]
pub(crate) async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "status": "ok", "service": "api-db" }))
}

pub fn router() -> Router<AppState> {
    let public = Router::new()
        .route("/health", axum::routing::get(health))
        .route("/login", axum::routing::post(login_handler))
        .route("/register", axum::routing::post(register_handler))
        .merge(email::routes());

    let protected = Router::new()
        .merge(usuario::routes())
        .merge(entrenamiento::routes())
        .merge(golpe::routes())
        .merge(historial::routes())
        .merge(rutina::routes())
        .merge(resultado_fuerza::routes())
        .route("/ws", axum::routing::get(ws::ws_handler))
        .layer(middleware::from_fn(auth_middleware));

    public.merge(protected)
}

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    paths(
        health,
        crate::auth::login_handler,
        crate::auth::register_handler,
        // Email
        email::resend_confirmation,
        email::confirm_email,
        email::forgot_password,
        email::reset_password_form,
        email::reset_password_submit,
        // Users
        usuario::list_users,
        usuario::get_user,
        usuario::create_user,
        usuario::update_user,
        usuario::delete_user,
        // Trainings
        entrenamiento::list_trainings,
        entrenamiento::get_training,
        entrenamiento::create_training,
        entrenamiento::update_training,
        entrenamiento::delete_training,
        entrenamiento::list_trainings_by_user,
        // Punches
        golpe::list_punches,
        golpe::get_punch,
        golpe::create_punch,
        golpe::update_punch,
        golpe::delete_punch,
        // History
        historial::list_history,
        historial::get_history,
        historial::create_history,
        historial::update_history,
        historial::delete_history,
        historial::list_history_by_training,
        // Routines
        rutina::list_routines,
        rutina::get_routine,
        rutina::create_routine,
        rutina::update_routine,
        rutina::delete_routine,
        // Force Results
        resultado_fuerza::list_results,
        resultado_fuerza::get_result,
        resultado_fuerza::create_result,
        resultado_fuerza::list_results_by_user,
    ),
    components(schemas(
        Usuario, CreateUsuario, UpdateUsuario,
        Entrenamiento, CreateEntrenamiento, UpdateEntrenamiento,
        Golpe, CreateGolpe, UpdateGolpe,
        Rutina, CreateRutina, UpdateRutina,
        Historial, CreateHistorial, UpdateHistorial, HistorialDetail,
        ResultadoFuerza, CreateResultadoFuerza,
        LoginRequest, LoginResponse, TokenClaims,
        email::ResendRequest, email::ResendResponse,
        email::ForgotPasswordRequest, email::ResetPasswordForm,
    )),
    tags(
        (name = "Health", description = "Health check"),
        (name = "Auth", description = "Authentication"),
        (name = "Email", description = "Email confirmation and password reset"),
        (name = "Users", description = "User management"),
        (name = "Trainings", description = "Training sessions"),
        (name = "Punches", description = "Punch catalog"),
        (name = "History", description = "Punch history"),
        (name = "Routines", description = "Training routines"),
        (name = "Force Results", description = "Force measurement results"),
    ),
    info(
        title = "KnockShadow API",
        version = "0.1.0",
        description = "KnockShadow boxing telemetry backend API",
    )
)]
pub struct ApiDoc;
