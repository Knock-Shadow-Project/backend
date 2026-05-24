use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};

use crate::models::{CreateEntrenamiento, Entrenamiento, Pagination, UpdateEntrenamiento};
use crate::state::AppState;

const ENTRENAMIENTO_COLUMNS: &str = "id_entrenamiento, id_usuario, id_rutina, hora_inicio, hora_fin, tipo, calorias, paso_actual, estado";

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

#[utoipa::path(get, path = "/trainings",
    params(Pagination),
    responses((status = 200, body = Vec<Entrenamiento>)),
    security(("bearer_auth" = [])),
    tag = "Trainings"
)]
pub(crate) async fn list_trainings(
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

#[utoipa::path(get, path = "/trainings/{id}",
    params(("id" = i32, Path,)),
    responses(
        (status = 200, body = Entrenamiento),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Trainings"
)]
pub(crate) async fn get_training(
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

#[utoipa::path(post, path = "/trainings",
    request_body = CreateEntrenamiento,
    responses((status = 200, body = Entrenamiento)),
    security(("bearer_auth" = [])),
    tag = "Trainings"
)]
pub(crate) async fn create_training(
    State(state): State<AppState>,
    Json(payload): Json<CreateEntrenamiento>,
) -> Result<Json<Entrenamiento>, StatusCode> {
    let query = format!(
        "INSERT INTO entrenamiento (id_usuario, id_rutina, hora_inicio, hora_fin, tipo, calorias, paso_actual, estado)
         VALUES (
            $1,
            $2,
            COALESCE($3, CURRENT_TIMESTAMP),
            $4,
            $5,
            COALESCE($6, 0),
            COALESCE($7, 0),
            COALESCE($8, 'ACTIVO')
         )
         RETURNING {cols}",
        cols = ENTRENAMIENTO_COLUMNS,
    );
    let item = sqlx::query_as::<_, Entrenamiento>(&query)
        .bind(payload.user_id)
        .bind(payload.routine_id)
        .bind(payload.start_time)
        .bind(payload.end_time)
        .bind(&payload.training_type)
        .bind(payload.calories)
        .bind(payload.current_step)
        .bind(&payload.state)
        .fetch_one(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create training: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(item))
}

#[utoipa::path(put, path = "/trainings/{id}",
    params(("id" = i32, Path,)),
    request_body = UpdateEntrenamiento,
    responses(
        (status = 200, body = Entrenamiento),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Trainings"
)]
pub(crate) async fn update_training(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateEntrenamiento>,
) -> Result<Json<Entrenamiento>, StatusCode> {
    let query = format!(
        "UPDATE entrenamiento SET
            id_usuario = COALESCE($1, id_usuario),
            id_rutina = COALESCE($2, id_rutina),
            hora_inicio = COALESCE($3, hora_inicio),
            hora_fin = COALESCE($4, hora_fin),
            tipo = COALESCE($5, tipo),
            calorias = COALESCE($6, calorias),
            paso_actual = COALESCE($7, paso_actual),
            estado = COALESCE($8, estado)
         WHERE id_entrenamiento = $9
         RETURNING {cols}",
        cols = ENTRENAMIENTO_COLUMNS,
    );
    let item = sqlx::query_as::<_, Entrenamiento>(&query)
        .bind(payload.user_id)
        .bind(payload.routine_id)
        .bind(payload.start_time)
        .bind(payload.end_time)
        .bind(&payload.training_type)
        .bind(payload.calories)
        .bind(payload.current_step)
        .bind(&payload.state)
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

#[utoipa::path(delete, path = "/trainings/{id}",
    params(("id" = i32, Path,)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Trainings"
)]
pub(crate) async fn delete_training(
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

#[utoipa::path(get, path = "/users/{id}/trainings",
    params(("id" = i32, Path,), Pagination),
    responses((status = 200, body = Vec<Entrenamiento>)),
    security(("bearer_auth" = [])),
    tag = "Trainings"
)]
pub(crate) async fn list_trainings_by_user(
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
