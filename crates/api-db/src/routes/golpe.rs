use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::models::{CreateGolpe, Golpe, UpdateGolpe};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/golpes", get(list_golpes).post(create_golpe))
        .route(
            "/golpes/{id}",
            get(get_golpe).put(update_golpe).delete(delete_golpe),
        )
}

async fn list_golpes(State(state): State<AppState>) -> Result<Json<Vec<Golpe>>, StatusCode> {
    let items = sqlx::query_as::<_, Golpe>("SELECT * FROM golpe")
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list golpes: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(items))
}

async fn get_golpe(
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
                tracing::error!("Failed to get golpe: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(item))
}

async fn create_golpe(
    State(state): State<AppState>,
    Json(payload): Json<CreateGolpe>,
) -> Result<Json<Golpe>, StatusCode> {
    let item = sqlx::query_as::<_, Golpe>(
        "INSERT INTO golpe (nombre, extremidad, posicion)
         VALUES ($1, $2, $3)
         RETURNING *",
    )
    .bind(&payload.nombre)
    .bind(&payload.extremidad)
    .bind(&payload.posicion)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create golpe: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(item))
}

async fn update_golpe(
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
    .bind(&payload.nombre)
    .bind(&payload.extremidad)
    .bind(&payload.posicion)
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to update golpe: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(item))
}

async fn delete_golpe(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM golpe WHERE id_golpe = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete golpe: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
