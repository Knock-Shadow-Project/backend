use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};

use crate::models::{CreateGolpe, Golpe, Pagination, UpdateGolpe};
use crate::state::AppState;

const GOLPE_COLUMNS: &str = "id_golpe, nombre, extremidad, posicion";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/punches", get(list_punches).post(create_punch))
        .route(
            "/punches/{id}",
            get(get_punch).put(update_punch).delete(delete_punch),
        )
}

#[utoipa::path(get, path = "/punches",
    params(Pagination),
    responses((status = 200, body = Vec<Golpe>)),
    security(("bearer_auth" = [])),
    tag = "Punches"
)]
pub(crate) async fn list_punches(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Golpe>>, StatusCode> {
    let limit = pagination.resolved_limit();
    let offset = pagination.resolved_offset();
    let query = format!(
        "SELECT {cols} FROM golpe ORDER BY id_golpe ASC LIMIT $1 OFFSET $2",
        cols = GOLPE_COLUMNS,
    );
    let items = sqlx::query_as::<_, Golpe>(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list punches: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(items))
}

#[utoipa::path(get, path = "/punches/{id}",
    params(("id" = i32, Path,)),
    responses(
        (status = 200, body = Golpe),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Punches"
)]
pub(crate) async fn get_punch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Golpe>, StatusCode> {
    let item = sqlx::query_as::<_, Golpe>("SELECT * FROM golpe WHERE id_golpe = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => {
                tracing::error!("Failed to get punch: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(item))
}

#[utoipa::path(post, path = "/punches",
    request_body = CreateGolpe,
    responses((status = 200, body = Golpe)),
    security(("bearer_auth" = [])),
    tag = "Punches"
)]
pub(crate) async fn create_punch(
    State(state): State<AppState>,
    Json(payload): Json<CreateGolpe>,
) -> Result<Json<Golpe>, StatusCode> {
    let item = sqlx::query_as::<_, Golpe>(
        "INSERT INTO golpe (nombre, extremidad, posicion)
         VALUES ($1, $2, $3)
         RETURNING id_golpe, nombre, extremidad, posicion",
    )
    .bind(&payload.name)
    .bind(&payload.limb)
    .bind(&payload.position)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create punch: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(item))
}

#[utoipa::path(put, path = "/punches/{id}",
    params(("id" = i32, Path,)),
    request_body = UpdateGolpe,
    responses(
        (status = 200, body = Golpe),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Punches"
)]
pub(crate) async fn update_punch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateGolpe>,
) -> Result<Json<Golpe>, StatusCode> {
    let item = sqlx::query_as::<_, Golpe>(
        "UPDATE golpe SET
            nombre = COALESCE($1, nombre),
            extremidad = COALESCE($2, extremidad),
            posicion = COALESCE($3, posicion)
         WHERE id_golpe = $4
         RETURNING *",
    )
    .bind(&payload.name)
    .bind(&payload.limb)
    .bind(&payload.position)
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to update punch: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(item))
}

#[utoipa::path(delete, path = "/punches/{id}",
    params(("id" = i32, Path,)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Punches"
)]
pub(crate) async fn delete_punch(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM golpe WHERE id_golpe = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete punch: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
