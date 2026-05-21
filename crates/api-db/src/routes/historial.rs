use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};

use crate::models::{CreateHistorial, Historial, HistorialDetail, Pagination, UpdateHistorial};
use crate::state::AppState;

/// Columnas planas de HISTORIAL. Tras la migración 002 la PK es `id_historial`
/// (antes era el par `(id_entrenamiento, id_golpe)`).
const HISTORIAL_COLUMNS: &str = "id_historial, id_entrenamiento, id_golpe_lanzado, id_golpe_esperado, potencia, es_correcto, fecha_impacto";

/// Selección con JOIN al catálogo GOLPE para el endpoint de lectura
/// enriquecida (`HistorialDetail`). Se alinea con las renombras de la
/// migración 002: `id_golpe_lanzado` es el golpe efectivamente detectado.
const HISTORIAL_DETAIL_SELECT: &str = "h.id_historial, h.id_entrenamiento, h.id_golpe_lanzado, h.id_golpe_esperado, h.potencia, h.es_correcto, h.fecha_impacto, g.nombre, g.extremidad, g.posicion";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/history", get(list_history).post(create_history))
        .route(
            "/history/{history_id}",
            get(get_history).put(update_history).delete(delete_history),
        )
        .route("/trainings/{id}/history", get(list_history_by_training))
}

async fn list_history(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<HistorialDetail>>, StatusCode> {
    // historial puede crecer ilimitadamente (1 fila por golpe registrado).
    // Pagar página por página evita devolver megabytes en un solo endpoint.
    let limit = pagination.resolved_limit();
    let offset = pagination.resolved_offset();
    let query = format!(
        "SELECT {sel}
         FROM historial h
         JOIN golpe g ON h.id_golpe_lanzado = g.id_golpe
         ORDER BY h.fecha_impacto DESC, h.id_historial DESC
         LIMIT $1 OFFSET $2",
        sel = HISTORIAL_DETAIL_SELECT,
    );
    let items = sqlx::query_as::<_, HistorialDetail>(&query)
        .bind(limit)
        .bind(offset)
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
    Path(history_id): Path<i32>,
) -> Result<Json<Historial>, StatusCode> {
    let query = format!(
        "SELECT {cols} FROM historial WHERE id_historial = $1",
        cols = HISTORIAL_COLUMNS,
    );
    let item = sqlx::query_as::<_, Historial>(&query)
        .bind(history_id)
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
    // Los DEFAULT de PostgreSQL (es_correcto=TRUE, fecha_impacto=CURRENT_TIMESTAMP)
    // se preservan vía COALESCE cuando el cliente no manda valor.
    let query = format!(
        "INSERT INTO historial (id_entrenamiento, id_golpe_lanzado, id_golpe_esperado, potencia, es_correcto, fecha_impacto)
         VALUES ($1, $2, $3, $4, COALESCE($5, TRUE), COALESCE($6, CURRENT_TIMESTAMP))
         RETURNING {cols}",
        cols = HISTORIAL_COLUMNS,
    );
    let item = sqlx::query_as::<_, Historial>(&query)
        .bind(payload.training_id)
        .bind(payload.thrown_punch_id)
        .bind(payload.expected_punch_id)
        .bind(payload.power)
        .bind(payload.is_correct)
        .bind(payload.impact_date)
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
    Path(history_id): Path<i32>,
    Json(payload): Json<UpdateHistorial>,
) -> Result<Json<Historial>, StatusCode> {
    let query = format!(
        "UPDATE historial SET
            id_golpe_lanzado = COALESCE($1, id_golpe_lanzado),
            id_golpe_esperado = COALESCE($2, id_golpe_esperado),
            potencia = COALESCE($3, potencia),
            es_correcto = COALESCE($4, es_correcto),
            fecha_impacto = COALESCE($5, fecha_impacto)
         WHERE id_historial = $6
         RETURNING {cols}",
        cols = HISTORIAL_COLUMNS,
    );
    let item = sqlx::query_as::<_, Historial>(&query)
        .bind(payload.thrown_punch_id)
        .bind(payload.expected_punch_id)
        .bind(payload.power)
        .bind(payload.is_correct)
        .bind(payload.impact_date)
        .bind(history_id)
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
    Path(history_id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM historial WHERE id_historial = $1")
        .bind(history_id)
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
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<HistorialDetail>>, StatusCode> {
    let limit = pagination.resolved_limit();
    let offset = pagination.resolved_offset();
    let query = format!(
        "SELECT {sel}
         FROM historial h
         JOIN golpe g ON h.id_golpe_lanzado = g.id_golpe
         WHERE h.id_entrenamiento = $1
         ORDER BY h.fecha_impacto ASC, h.id_historial ASC
         LIMIT $2 OFFSET $3",
        sel = HISTORIAL_DETAIL_SELECT,
    );
    let items = sqlx::query_as::<_, HistorialDetail>(&query)
        .bind(id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list history by training: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(items))
}
