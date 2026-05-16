use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};

use crate::models::{CreateEntrenamiento, Entrenamiento, Pagination, UpdateEntrenamiento};
use crate::state::AppState;

/// Lista explícita de columnas a devolver en los endpoints de lectura.
///
/// Evitamos `SELECT *` por dos motivos:
///   1. Si el esquema gana columnas internas (audit, soft-delete, etc.)
///      no las exponemos accidentalmente al cliente.
///   2. El planner puede usar índices de cobertura cuando las columnas son
///      conocidas en tiempo de compilación.
const ENTRENAMIENTO_COLUMNS: &str =
    "id_entrenamiento, hora_inicio, hora_fin, tipo, calorias, id_usuario";

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/trainings", get(list_trainings).post(create_training))
        .route(
            "/trainings/{id}",
            get(get_training)
                .put(update_training)
                .delete(delete_training),
        )
        .route("/users/{id}/trainings", get(list_trainings_by_user))
}

async fn list_trainings(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Entrenamiento>>, StatusCode> {
    let limit = pagination.resolved_limit();
    let offset = pagination.resolved_offset();
    let query = format!(
        "SELECT {cols} FROM entrenamiento \
         ORDER BY hora_inicio DESC, id_entrenamiento DESC \
         LIMIT $1 OFFSET $2",
        cols = ENTRENAMIENTO_COLUMNS,
    );
    let items = sqlx::query_as::<_, Entrenamiento>(&query)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list trainings: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(items))
}

async fn get_training(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Entrenamiento>, StatusCode> {
    let query = format!(
        "SELECT {cols} FROM entrenamiento WHERE id_entrenamiento = $1",
        cols = ENTRENAMIENTO_COLUMNS,
    );
    let item = sqlx::query_as::<_, Entrenamiento>(&query)
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => {
                tracing::error!("Failed to get training: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(item))
}

async fn create_training(
    State(state): State<AppState>,
    Json(payload): Json<CreateEntrenamiento>,
) -> Result<Json<Entrenamiento>, StatusCode> {
    let query = format!(
        "INSERT INTO entrenamiento (hora_inicio, hora_fin, tipo, calorias, id_usuario)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING {cols}",
        cols = ENTRENAMIENTO_COLUMNS,
    );
    let item = sqlx::query_as::<_, Entrenamiento>(&query)
        .bind(payload.start_time)
        .bind(payload.end_time)
        .bind(&payload.training_type)
        .bind(payload.calories)
        .bind(payload.user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create training: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(item))
}

async fn update_training(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateEntrenamiento>,
) -> Result<Json<Entrenamiento>, StatusCode> {
    let query = format!(
        "UPDATE entrenamiento SET
            hora_inicio = COALESCE($1, hora_inicio),
            hora_fin = COALESCE($2, hora_fin),
            tipo = COALESCE($3, tipo),
            calorias = COALESCE($4, calorias),
            id_usuario = COALESCE($5, id_usuario)
         WHERE id_entrenamiento = $6
         RETURNING {cols}",
        cols = ENTRENAMIENTO_COLUMNS,
    );
    let item = sqlx::query_as::<_, Entrenamiento>(&query)
        .bind(payload.start_time)
        .bind(payload.end_time)
        .bind(&payload.training_type)
        .bind(payload.calories)
        .bind(payload.user_id)
        .bind(id)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
            _ => {
                tracing::error!("Failed to update training: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            }
        })?;
    Ok(Json(item))
}

async fn delete_training(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM entrenamiento WHERE id_entrenamiento = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete training: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_trainings_by_user(
    State(state): State<AppState>,
    Path(user_id): Path<i32>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Entrenamiento>>, StatusCode> {
    let limit = pagination.resolved_limit();
    let offset = pagination.resolved_offset();
    let query = format!(
        "SELECT {cols} FROM entrenamiento WHERE id_usuario = $1 \
         ORDER BY hora_inicio DESC, id_entrenamiento DESC \
         LIMIT $2 OFFSET $3",
        cols = ENTRENAMIENTO_COLUMNS,
    );
    let items = sqlx::query_as::<_, Entrenamiento>(&query)
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list trainings by user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(items))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Pagination::resolved_limit` debe forzar 50 cuando no se pasa nada y
    /// recortar a 200 cuando el cliente pide más, para evitar payloads enormes.
    #[test]
    fn pagination_clamps_limit() {
        assert_eq!(
            Pagination {
                limit: None,
                offset: None
            }
            .resolved_limit(),
            crate::models::DEFAULT_PAGE_SIZE
        );
        assert_eq!(
            Pagination {
                limit: Some(10_000),
                offset: None
            }
            .resolved_limit(),
            crate::models::MAX_PAGE_SIZE
        );
        assert_eq!(
            Pagination {
                limit: Some(0),
                offset: None
            }
            .resolved_limit(),
            1
        );
    }

    #[test]
    fn pagination_clamps_offset() {
        assert_eq!(
            Pagination {
                limit: None,
                offset: None
            }
            .resolved_offset(),
            0
        );
        assert_eq!(
            Pagination {
                limit: None,
                offset: Some(-5)
            }
            .resolved_offset(),
            0
        );
        assert_eq!(
            Pagination {
                limit: None,
                offset: Some(123)
            }
            .resolved_offset(),
            123
        );
    }
}
