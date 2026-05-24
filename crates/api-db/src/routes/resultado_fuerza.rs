use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};

use crate::models::{CreateResultadoFuerza, Pagination, ResultadoFuerza};
use crate::state::AppState;

const COLS: &str =
    "id_resultado, id_usuario, nombre_participante, puntuacion, modo, grupo, fecha";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/force-results", get(list_results).post(create_result))
        .route("/force-results/{id}", get(get_result))
        .route(
            "/users/{id}/force-results",
            get(list_results_by_user),
        )
}

#[utoipa::path(get, path = "/force-results",
    params(Pagination),
    responses((status = 200, body = Vec<ResultadoFuerza>)),
    security(("bearer_auth" = [])),
    tag = "Force Results"
)]
pub(crate) async fn list_results(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<ResultadoFuerza>>, StatusCode> {
    let limit = pagination.resolved_limit();
    let offset = pagination.resolved_offset();
    let query = format!(
        "SELECT {COLS} FROM resultado_fuerza ORDER BY fecha DESC LIMIT $1 OFFSET $2"
    );
    let items = sqlx::query_as::<_, ResultadoFuerza>(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list force results: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(items))
}

#[utoipa::path(get, path = "/force-results/{id}",
    params(("id" = i32, Path,)),
    responses(
        (status = 200, body = ResultadoFuerza),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Force Results"
)]
pub(crate) async fn get_result(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<ResultadoFuerza>, StatusCode> {
    let query = format!("SELECT {COLS} FROM resultado_fuerza WHERE id_resultado = $1");
    let item = sqlx::query_as::<_, ResultadoFuerza>(&query)
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => {
                tracing::error!("Failed to get force result: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(item))
}

#[utoipa::path(post, path = "/force-results",
    request_body = CreateResultadoFuerza,
    responses((status = 200, body = ResultadoFuerza)),
    security(("bearer_auth" = [])),
    tag = "Force Results"
)]
pub(crate) async fn create_result(
    State(state): State<AppState>,
    Json(payload): Json<CreateResultadoFuerza>,
) -> Result<Json<ResultadoFuerza>, StatusCode> {
    let query = format!(
        "INSERT INTO resultado_fuerza (id_usuario, nombre_participante, puntuacion, modo, grupo)
         VALUES ($1, $2, $3, COALESCE($4, 'Medidor'), $5)
         RETURNING {COLS}"
    );
    let item = sqlx::query_as::<_, ResultadoFuerza>(&query)
        .bind(payload.user_id)
        .bind(&payload.participant_name)
        .bind(payload.score)
        .bind(&payload.mode)
        .bind(&payload.group_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create force result: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(item))
}

#[utoipa::path(get, path = "/users/{id}/force-results",
    params(("id" = i32, Path,), Pagination),
    responses((status = 200, body = Vec<ResultadoFuerza>)),
    security(("bearer_auth" = [])),
    tag = "Force Results"
)]
pub(crate) async fn list_results_by_user(
    State(state): State<AppState>,
    Path(user_id): Path<i32>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<ResultadoFuerza>>, StatusCode> {
    let limit = pagination.resolved_limit();
    let offset = pagination.resolved_offset();
    let query = format!(
        "SELECT {COLS} FROM resultado_fuerza WHERE id_usuario = $1
         ORDER BY fecha DESC LIMIT $2 OFFSET $3"
    );
    let items = sqlx::query_as::<_, ResultadoFuerza>(&query)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list force results by user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(items))
}
