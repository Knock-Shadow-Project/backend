use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};

use crate::models::{CreateRutina, Pagination, Rutina, UpdateRutina};
use crate::state::AppState;

const RUTINA_COLUMNS: &str = "id_rutina, nombre, nivel_recomendado, secuencia_golpes";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/routines", get(list_routines).post(create_routine))
        .route(
            "/routines/{id}",
            get(get_routine).put(update_routine).delete(delete_routine),
        )
}

#[utoipa::path(get, path = "/routines",
    params(Pagination),
    responses((status = 200, body = Vec<Rutina>)),
    security(("bearer_auth" = [])),
    tag = "Routines"
)]
pub(crate) async fn list_routines(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Rutina>>, StatusCode> {
    let limit = pagination.resolved_limit();
    let offset = pagination.resolved_offset();
    let query = format!(
        "SELECT {cols} FROM rutina ORDER BY id_rutina ASC LIMIT $1 OFFSET $2",
        cols = RUTINA_COLUMNS,
    );
    let items = sqlx::query_as::<_, Rutina>(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list routines: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(items))
}

#[utoipa::path(get, path = "/routines/{id}",
    params(("id" = i32, Path,)),
    responses(
        (status = 200, body = Rutina),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Routines"
)]
pub(crate) async fn get_routine(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Rutina>, StatusCode> {
    let query = format!(
        "SELECT {cols} FROM rutina WHERE id_rutina = $1",
        cols = RUTINA_COLUMNS,
    );
    let item = sqlx::query_as::<_, Rutina>(&query)
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => {
                tracing::error!("Failed to get routine: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(item))
}

#[utoipa::path(post, path = "/routines",
    request_body = CreateRutina,
    responses((status = 200, body = Rutina)),
    security(("bearer_auth" = [])),
    tag = "Routines"
)]
pub(crate) async fn create_routine(
    State(state): State<AppState>,
    Json(payload): Json<CreateRutina>,
) -> Result<Json<Rutina>, StatusCode> {
    let query = format!(
        "INSERT INTO rutina (nombre, nivel_recomendado, secuencia_golpes)
         VALUES ($1, $2, $3)
         RETURNING {cols}",
        cols = RUTINA_COLUMNS,
    );
    let item = sqlx::query_as::<_, Rutina>(&query)
        .bind(&payload.name)
        .bind(&payload.recommended_level)
        .bind(payload.punch_sequence.as_deref())
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create routine: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(item))
}

#[utoipa::path(put, path = "/routines/{id}",
    params(("id" = i32, Path,)),
    request_body = UpdateRutina,
    responses(
        (status = 200, body = Rutina),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Routines"
)]
pub(crate) async fn update_routine(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateRutina>,
) -> Result<Json<Rutina>, StatusCode> {
    let query = format!(
        "UPDATE rutina SET
            nombre = COALESCE($1, nombre),
            nivel_recomendado = COALESCE($2, nivel_recomendado),
            secuencia_golpes = COALESCE($3, secuencia_golpes)
         WHERE id_rutina = $4
         RETURNING {cols}",
        cols = RUTINA_COLUMNS,
    );
    let item = sqlx::query_as::<_, Rutina>(&query)
        .bind(&payload.name)
        .bind(&payload.recommended_level)
        .bind(payload.punch_sequence.as_deref())
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => {
                tracing::error!("Failed to update routine: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(item))
}

#[utoipa::path(delete, path = "/routines/{id}",
    params(("id" = i32, Path,)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Routines"
)]
pub(crate) async fn delete_routine(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM rutina WHERE id_rutina = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete routine: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
