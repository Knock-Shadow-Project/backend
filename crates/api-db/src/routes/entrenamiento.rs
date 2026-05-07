use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::models::{CreateEntrenamiento, Entrenamiento, UpdateEntrenamiento};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/trainings",
            get(list_trainings).post(create_training),
        )
        .route(
            "/trainings/{id}",
            get(get_training)
                .put(update_training)
                .delete(delete_training),
        )
        .route(
            "/users/{id}/trainings",
            get(list_trainings_by_user),
        )
}

async fn list_trainings(
    State(state): State<AppState>,
) -> Result<Json<Vec<Entrenamiento>>, StatusCode> {
    let items = sqlx::query_as::<_, Entrenamiento>("SELECT * FROM entrenamiento")
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
    let item = sqlx::query_as::<_, Entrenamiento>(
        "SELECT * FROM entrenamiento WHERE id_entrenamiento = $1",
    )
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
    let item = sqlx::query_as::<_, Entrenamiento>(
        "INSERT INTO entrenamiento (hora_inicio, hora_fin, tipo, calorias, id_usuario)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
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
    let item = sqlx::query_as::<_, Entrenamiento>(
        "UPDATE entrenamiento SET
            hora_inicio = COALESCE($1, hora_inicio),
            hora_fin = COALESCE($2, hora_fin),
            tipo = COALESCE($3, tipo),
            calorias = COALESCE($4, calorias),
            id_usuario = COALESCE($5, id_usuario)
         WHERE id_entrenamiento = $6
         RETURNING *",
    )
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
) -> Result<Json<Vec<Entrenamiento>>, StatusCode> {
    let items = sqlx::query_as::<_, Entrenamiento>(
        "SELECT * FROM entrenamiento WHERE id_usuario = $1",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list trainings by user: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(items))
}
