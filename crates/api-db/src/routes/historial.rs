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
        .route("/historial", get(list_historial).post(create_historial))
        .route(
            "/historial/{id_entrenamiento}/{id_golpe}",
            get(get_historial)
                .put(update_historial)
                .delete(delete_historial),
        )
        .route(
            "/entrenamientos/{id}/historial",
            get(list_historial_by_entrenamiento),
        )
}

async fn list_historial(
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
        tracing::error!("Failed to list historial: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(items))
}

async fn get_historial(
    State(state): State<AppState>,
    Path((id_entrenamiento, id_golpe)): Path<(i32, i32)>,
) -> Result<Json<Historial>, StatusCode> {
    let item = sqlx::query_as::<_, Historial>(
        "SELECT * FROM historial WHERE id_entrenamiento = $1 AND id_golpe = $2",
    )
    .bind(id_entrenamiento)
    .bind(id_golpe)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to get historial: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(item))
}

async fn create_historial(
    State(state): State<AppState>,
    Json(payload): Json<CreateHistorial>,
) -> Result<Json<Historial>, StatusCode> {
    let item = sqlx::query_as::<_, Historial>(
        "INSERT INTO historial (id_entrenamiento, id_golpe, potencia)
         VALUES ($1, $2, $3)
         RETURNING *",
    )
    .bind(payload.id_entrenamiento)
    .bind(payload.id_golpe)
    .bind(payload.potencia)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create historial: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(item))
}

async fn update_historial(
    State(state): State<AppState>,
    Path((id_entrenamiento, id_golpe)): Path<(i32, i32)>,
    Json(payload): Json<UpdateHistorial>,
) -> Result<Json<Historial>, StatusCode> {
    let item = sqlx::query_as::<_, Historial>(
        "UPDATE historial SET
            potencia = COALESCE($1, potencia)
         WHERE id_entrenamiento = $2 AND id_golpe = $3
         RETURNING *",
    )
    .bind(payload.potencia)
    .bind(id_entrenamiento)
    .bind(id_golpe)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to update historial: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(item))
}

async fn delete_historial(
    State(state): State<AppState>,
    Path((id_entrenamiento, id_golpe)): Path<(i32, i32)>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query(
        "DELETE FROM historial WHERE id_entrenamiento = $1 AND id_golpe = $2",
    )
    .bind(id_entrenamiento)
    .bind(id_golpe)
    .execute(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to delete historial: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_historial_by_entrenamiento(
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
        tracing::error!("Failed to list historial by entrenamiento: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(items))
}
