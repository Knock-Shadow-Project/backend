use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::models::{CreateHistorial, Historial, HistorialDetail, UpdateHistorial};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/history", get(list_history).post(create_history))
        .route(
            "/history/{training_id}/{punch_id}",
            get(get_history)
                .put(update_history)
                .delete(delete_history),
        )
        .route(
            "/trainings/{id}/history",
            get(list_history_by_training),
        )
}

async fn list_history(
    State(state): State<AppState>,
) -> Result<Json<Vec<HistorialDetail>>, StatusCode> {
    let items = sqlx::query_as::<_, HistorialDetail>(
        "SELECT h.id_entrenamiento, h.id_golpe, h.potencia, g.nombre, g.extremidad, g.posicion
         FROM historial h
         JOIN golpe g ON h.id_golpe = g.id_golpe",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list history: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(items))
}

async fn get_history(
    State(state): State<AppState>,
    Path((training_id, punch_id)): Path<(i32, i32)>,
) -> Result<Json<Historial>, StatusCode> {
    let item = sqlx::query_as::<_, Historial>(
        "SELECT * FROM historial WHERE id_entrenamiento = $1 AND id_golpe = $2",
    )
    .bind(training_id)
    .bind(punch_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to get history: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(item))
}

async fn create_history(
    State(state): State<AppState>,
    Json(payload): Json<CreateHistorial>,
) -> Result<Json<Historial>, StatusCode> {
    let item = sqlx::query_as::<_, Historial>(
        "INSERT INTO historial (id_entrenamiento, id_golpe, potencia)
         VALUES ($1, $2, $3)
         RETURNING *",
    )
    .bind(payload.training_id)
    .bind(payload.punch_id)
    .bind(payload.power)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create history: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(item))
}

async fn update_history(
    State(state): State<AppState>,
    Path((training_id, punch_id)): Path<(i32, i32)>,
    Json(payload): Json<UpdateHistorial>,
) -> Result<Json<Historial>, StatusCode> {
    let item = sqlx::query_as::<_, Historial>(
        "UPDATE historial SET
            potencia = COALESCE($1, potencia)
         WHERE id_entrenamiento = $2 AND id_golpe = $3
         RETURNING *",
    )
    .bind(payload.power)
    .bind(training_id)
    .bind(punch_id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to update history: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(item))
}

async fn delete_history(
    State(state): State<AppState>,
    Path((training_id, punch_id)): Path<(i32, i32)>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query(
        "DELETE FROM historial WHERE id_entrenamiento = $1 AND id_golpe = $2",
    )
    .bind(training_id)
    .bind(punch_id)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete history: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_history_by_training(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Vec<HistorialDetail>>, StatusCode> {
    let items = sqlx::query_as::<_, HistorialDetail>(
        "SELECT h.id_entrenamiento, h.id_golpe, h.potencia, g.nombre, g.extremidad, g.posicion
         FROM historial h
         JOIN golpe g ON h.id_golpe = g.id_golpe
         WHERE h.id_entrenamiento = $1",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list history by training: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(items))
}
