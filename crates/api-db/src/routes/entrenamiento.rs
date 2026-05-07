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
            "/entrenamientos",
            get(list_entrenamientos).post(create_entrenamiento),
        )
        .route(
            "/entrenamientos/{id}",
            get(get_entrenamiento)
                .put(update_entrenamiento)
                .delete(delete_entrenamiento),
        )
        .route(
            "/usuarios/{id}/entrenamientos",
            get(list_entrenamientos_by_usuario),
        )
}

async fn list_entrenamientos(
    State(state): State<AppState>,
) -> Result<Json<Vec<Entrenamiento>>, StatusCode> {
    let items = sqlx::query_as::<_, Entrenamiento>("SELECT * FROM entrenamiento")
        .fetch_all(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list entrenamientos: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    Ok(Json(items))
}

async fn get_entrenamiento(
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
            tracing::error!("Failed to get entrenamiento: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(item))
}

async fn create_entrenamiento(
    State(state): State<AppState>,
    Json(payload): Json<CreateEntrenamiento>,
) -> Result<Json<Entrenamiento>, StatusCode> {
    let item = sqlx::query_as::<_, Entrenamiento>(
        "INSERT INTO entrenamiento (hora_inicio, hora_fin, tipo, calorias, id_usuario)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING *",
    )
    .bind(payload.hora_inicio)
    .bind(payload.hora_fin)
    .bind(&payload.tipo)
    .bind(payload.calorias)
    .bind(payload.id_usuario)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create entrenamiento: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(item))
}

async fn update_entrenamiento(
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
    .bind(payload.hora_inicio)
    .bind(payload.hora_fin)
    .bind(&payload.tipo)
    .bind(payload.calorias)
    .bind(payload.id_usuario)
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to update entrenamiento: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(item))
}

async fn delete_entrenamiento(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM entrenamiento WHERE id_entrenamiento = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete entrenamiento: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn list_entrenamientos_by_usuario(
    State(state): State<AppState>,
    Path(id_usuario): Path<i32>,
) -> Result<Json<Vec<Entrenamiento>>, StatusCode> {
    let items = sqlx::query_as::<_, Entrenamiento>(
        "SELECT * FROM entrenamiento WHERE id_usuario = $1",
    )
    .bind(id_usuario)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list entrenamientos by usuario: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(items))
}
