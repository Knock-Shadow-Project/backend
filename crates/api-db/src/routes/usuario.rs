use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};

use crate::auth::hash_password;
use crate::models::{CreateUsuario, Pagination, UpdateUsuario, Usuario};
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users).post(create_user))
        .route(
            "/users/{id}",
            get(get_user).put(update_user).delete(delete_user),
        )
}

#[utoipa::path(get, path = "/users",
    params(Pagination),
    responses((status = 200, body = Vec<Usuario>)),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub(crate) async fn list_users(
    State(state): State<AppState>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Usuario>>, StatusCode> {
    let limit = pagination.resolved_limit();
    let offset = pagination.resolved_offset();
    let usuarios = sqlx::query_as::<_, Usuario>(
        "SELECT id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel, confirmado \
         FROM usuario \
         ORDER BY id_usuario ASC \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to list users: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(usuarios))
}

#[utoipa::path(get, path = "/users/{id}",
    params(("id" = i32, Path,)),
    responses(
        (status = 200, body = Usuario),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub(crate) async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<Json<Usuario>, StatusCode> {
    let usuario = sqlx::query_as::<_, Usuario>(
        "SELECT id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel, confirmado FROM usuario WHERE id_usuario = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to get user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(usuario))
}

#[utoipa::path(post, path = "/users",
    request_body = CreateUsuario,
    responses(
        (status = 200, body = Usuario),
        (status = 409, description = "Email already registered"),
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub(crate) async fn create_user(
    State(state): State<AppState>,
    Json(payload): Json<CreateUsuario>,
) -> Result<Json<Usuario>, (StatusCode, Json<serde_json::Value>)> {
    let hashed_password = hash_password(&payload.password).map_err(|e| {
        tracing::error!("Password hash error: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "message": "Error interno al cifrar la contraseña" })),
        )
    })?;

    let usuario = sqlx::query_as::<_, Usuario>(
        "INSERT INTO usuario (nombre, apellido, correo, contrasena, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
         RETURNING id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel, confirmado",
    )
    .bind(&payload.first_name)
    .bind(&payload.last_name)
    .bind(&payload.email)
    .bind(&hashed_password)
    .bind(&payload.phone)
    .bind(payload.age)
    .bind(payload.weight)
    .bind(payload.height)
    .bind(&payload.country)
    .bind(&payload.city)
    .bind(&payload.address)
    .bind(&payload.laterality)
    .bind(&payload.level)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(db_err) = &e
            && db_err.code().as_deref() == Some("23505")
        {
            return (
                StatusCode::CONFLICT,
                Json(serde_json::json!({
                    "message": "El correo ya está registrado",
                    "code": "EMAIL_TAKEN"
                })),
            );
        }
        tracing::error!("Failed to create user: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "message": "Error interno al crear usuario" })),
        )
    })?;
    Ok(Json(usuario))
}

#[utoipa::path(put, path = "/users/{id}",
    params(("id" = i32, Path,)),
    request_body = UpdateUsuario,
    responses(
        (status = 200, body = Usuario),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub(crate) async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateUsuario>,
) -> Result<Json<Usuario>, StatusCode> {
    let hashed_password = match payload.password {
        Some(ref pwd) => Some(hash_password(pwd).map_err(|e| {
            tracing::error!("Password hash error: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?),
        None => None,
    };

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
         RETURNING id_usuario, nombre, apellido, correo, telefono, edad, peso, estatura, pais, ciudad, direccion, lateralidad, nivel, confirmado",
    )
    .bind(&payload.first_name)
    .bind(&payload.last_name)
    .bind(&payload.email)
    .bind(&hashed_password)
    .bind(&payload.phone)
    .bind(payload.age)
    .bind(payload.weight)
    .bind(payload.height)
    .bind(&payload.country)
    .bind(&payload.city)
    .bind(&payload.address)
    .bind(&payload.laterality)
    .bind(&payload.level)
    .bind(id)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => StatusCode::NOT_FOUND,
        _ => {
            tracing::error!("Failed to update user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    })?;
    Ok(Json(usuario))
}

#[utoipa::path(delete, path = "/users/{id}",
    params(("id" = i32, Path,)),
    responses(
        (status = 204, description = "Deleted"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = [])),
    tag = "Users"
)]
pub(crate) async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<i32>,
) -> Result<StatusCode, StatusCode> {
    let result = sqlx::query("DELETE FROM usuario WHERE id_usuario = $1")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to delete user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
