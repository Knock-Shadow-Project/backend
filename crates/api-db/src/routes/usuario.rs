use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};

use crate::models::{CreateUsuario, UpdateUsuario, Usuario};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/usuarios", get(list_usuarios).post(create_usuario))
        .route(
            "/usuarios/{id}",
            get(get_usuario).put(update_usuario).delete(delete_usuario),
        )
}

async fn list_usuarios(State(state): State<AppState>) -> Result<Json<Vec<Usuario>>, StatusCode> {
    let usuarios = sqlx::query_as::<_, Usuario>(
        "SELECT id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel FROM usuario",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list usuarios: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(usuarios))
}

async fn get_usuario(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Usuario>, StatusCode> {
    let usuario = sqlx::query_as::<_, Usuario>(
        "SELECT id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel FROM usuario WHERE id_usuario = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to get usuario: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(usuario))
}

async fn create_usuario(
    State(state): State<AppState>,
    Json(payload): Json<CreateUsuario>,
) -> Result<Json<Usuario>, StatusCode> {
    let usuario = sqlx::query_as::<_, Usuario>(
        "INSERT INTO usuario (nombre, apellido, correo, contrasena, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel",
    )
    .bind(&payload.nombre)
    .bind(&payload.apellido)
    .bind(&payload.correo)
    .bind(&payload.contrasena)
    .bind(&payload.telefono)
    .bind(payload.edad)
    .bind(payload.peso)
    .bind(payload.estatura)
    .bind(&payload.pais)
    .bind(&payload.ciudad)
    .bind(&payload.direccion)
    .bind(&payload.lateralidad)
    .bind(&payload.nivel)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to create usuario: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(usuario))
}

async fn update_usuario(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateUsuario>,
) -> Result<Json<Usuario>, StatusCode> {
    let usuario = sqlx::query_as::<_, Usuario>(
        "UPDATE usuario SET
            nombre = COALESCE($1, nombre),
            apellido = COALESCE($2, apellido),
            correo = COALESCE($3, correo),
            contrasena = COALESCE($4, contrasena),
            telefono = COALESCE($5, telefono),
            edad = COALESCE($6, edad),
            peso = COALESCE($7, peso),
            estatura = COALESCE($8, estatura),
            pais = COALESCE($9, pais),
            ciudad = COALESCE($10, ciudad),
            direccion = COALESCE($11, direccion),
            lateralidad = COALESCE($12, lateralidad),
            nivel = COALESCE($13, nivel)
         WHERE id_usuario = $14
         RETURNING id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel",
    )
    .bind(&payload.nombre)
    .bind(&payload.apellido)
    .bind(&payload.correo)
    .bind(&payload.contrasena)
    .bind(&payload.telefono)
    .bind(payload.edad)
    .bind(payload.peso)
    .bind(payload.estatura)
    .bind(&payload.pais)
    .bind(&payload.ciudad)
    .bind(&payload.direccion)
    .bind(&payload.lateralidad)
    .bind(&payload.nivel)
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to update usuario: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(usuario))
}

async fn delete_usuario(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM usuario WHERE id_usuario = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete usuario: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
