use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Usuario {
    #[serde(rename = "user_id")]
    #[sqlx(rename = "id_usuario")]
    pub user_id: i32,
    #[serde(rename = "first_name")]
    #[sqlx(rename = "nombre")]
    pub first_name: String,
    #[serde(rename = "last_name")]
    #[sqlx(rename = "apellido")]
    pub last_name: String,
    #[serde(rename = "email")]
    #[sqlx(rename = "correo")]
    pub email: String,
    #[serde(rename = "phone")]
    #[sqlx(rename = "telefono")]
    pub phone: Option<String>,
    #[serde(rename = "age")]
    #[sqlx(rename = "edad")]
    pub age: Option<i32>,
    #[serde(rename = "weight")]
    #[sqlx(rename = "peso")]
    pub weight: Option<BigDecimal>,
    #[serde(rename = "height")]
    #[sqlx(rename = "estatura")]
    pub height: Option<i32>,
    #[serde(rename = "country")]
    #[sqlx(rename = "pais")]
    pub country: Option<String>,
    #[serde(rename = "city")]
    #[sqlx(rename = "ciudad")]
    pub city: Option<String>,
    #[serde(rename = "address")]
    #[sqlx(rename = "direccion")]
    pub address: Option<String>,
    #[serde(rename = "laterality")]
    #[sqlx(rename = "lateralidad")]
    pub laterality: Option<String>,
    #[serde(rename = "level")]
    #[sqlx(rename = "nivel")]
    pub level: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUsuario {
    #[serde(rename = "first_name")]
    pub first_name: String,
    #[serde(rename = "last_name")]
    pub last_name: String,
    #[serde(rename = "email")]
    pub email: String,
    #[serde(rename = "password")]
    pub password: String,
    #[serde(rename = "phone")]
    pub phone: Option<String>,
    #[serde(rename = "age")]
    pub age: Option<i32>,
    #[serde(rename = "weight")]
    pub weight: Option<BigDecimal>,
    #[serde(rename = "height")]
    pub height: Option<i32>,
    #[serde(rename = "country")]
    pub country: Option<String>,
    #[serde(rename = "city")]
    pub city: Option<String>,
    #[serde(rename = "address")]
    pub address: Option<String>,
    #[serde(rename = "laterality")]
    pub laterality: Option<String>,
    #[serde(rename = "level")]
    pub level: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUsuario {
    #[serde(rename = "first_name")]
    pub first_name: Option<String>,
    #[serde(rename = "last_name")]
    pub last_name: Option<String>,
    #[serde(rename = "email")]
    pub email: Option<String>,
    #[serde(rename = "password")]
    pub password: Option<String>,
    #[serde(rename = "phone")]
    pub phone: Option<String>,
    #[serde(rename = "age")]
    pub age: Option<i32>,
    #[serde(rename = "weight")]
    pub weight: Option<BigDecimal>,
    #[serde(rename = "height")]
    pub height: Option<i32>,
    #[serde(rename = "country")]
    pub country: Option<String>,
    #[serde(rename = "city")]
    pub city: Option<String>,
    #[serde(rename = "address")]
    pub address: Option<String>,
    #[serde(rename = "laterality")]
    pub laterality: Option<String>,
    #[serde(rename = "level")]
    pub level: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Entrenamiento {
    #[serde(rename = "training_id")]
    #[sqlx(rename = "id_entrenamiento")]
    pub training_id: i32,
    #[serde(rename = "start_time")]
    #[sqlx(rename = "hora_inicio")]
    pub start_time: NaiveDateTime,
    #[serde(rename = "end_time")]
    #[sqlx(rename = "hora_fin")]
    pub end_time: Option<NaiveDateTime>,
    #[serde(rename = "training_type")]
    #[sqlx(rename = "tipo")]
    pub training_type: Option<String>,
    #[serde(rename = "calories")]
    #[sqlx(rename = "calorias")]
    pub calories: Option<i32>,
    #[serde(rename = "user_id")]
    #[sqlx(rename = "id_usuario")]
    pub user_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateEntrenamiento {
    #[serde(rename = "start_time")]
    pub start_time: NaiveDateTime,
    #[serde(rename = "end_time")]
    pub end_time: Option<NaiveDateTime>,
    #[serde(rename = "training_type")]
    pub training_type: Option<String>,
    #[serde(rename = "calories")]
    pub calories: Option<i32>,
    #[serde(rename = "user_id")]
    pub user_id: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEntrenamiento {
    #[serde(rename = "start_time")]
    pub start_time: Option<NaiveDateTime>,
    #[serde(rename = "end_time")]
    pub end_time: Option<NaiveDateTime>,
    #[serde(rename = "training_type")]
    pub training_type: Option<String>,
    #[serde(rename = "calories")]
    pub calories: Option<i32>,
    #[serde(rename = "user_id")]
    pub user_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Golpe {
    #[serde(rename = "punch_id")]
    #[sqlx(rename = "id_golpe")]
    pub punch_id: i32,
    #[serde(rename = "name")]
    #[sqlx(rename = "nombre")]
    pub name: String,
    #[serde(rename = "limb")]
    #[sqlx(rename = "extremidad")]
    pub limb: Option<String>,
    #[serde(rename = "position")]
    #[sqlx(rename = "posicion")]
    pub position: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGolpe {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "limb")]
    pub limb: Option<String>,
    #[serde(rename = "position")]
    pub position: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGolpe {
    #[serde(rename = "name")]
    pub name: Option<String>,
    #[serde(rename = "limb")]
    pub limb: Option<String>,
    #[serde(rename = "position")]
    pub position: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Historial {
    #[serde(rename = "training_id")]
    #[sqlx(rename = "id_entrenamiento")]
    pub training_id: i32,
    #[serde(rename = "punch_id")]
    #[sqlx(rename = "id_golpe")]
    pub punch_id: i32,
    #[serde(rename = "power")]
    #[sqlx(rename = "potencia")]
    pub power: Option<BigDecimal>,
}

#[derive(Debug, Deserialize)]
pub struct CreateHistorial {
    #[serde(rename = "training_id")]
    pub training_id: i32,
    #[serde(rename = "punch_id")]
    pub punch_id: i32,
    #[serde(rename = "power")]
    pub power: Option<BigDecimal>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHistorial {
    #[serde(rename = "power")]
    pub power: Option<BigDecimal>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct HistorialDetail {
    #[serde(rename = "training_id")]
    #[sqlx(rename = "id_entrenamiento")]
    pub training_id: i32,
    #[serde(rename = "punch_id")]
    #[sqlx(rename = "id_golpe")]
    pub punch_id: i32,
    #[serde(rename = "power")]
    #[sqlx(rename = "potencia")]
    pub power: Option<BigDecimal>,
    #[serde(rename = "name")]
    #[sqlx(rename = "nombre")]
    pub name: String,
    #[serde(rename = "limb")]
    #[sqlx(rename = "extremidad")]
    pub limb: Option<String>,
    #[serde(rename = "position")]
    #[sqlx(rename = "posicion")]
    pub position: Option<String>,
}

// Auth models ---------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    #[serde(rename = "email")]
    pub email: String,
    #[serde(rename = "password")]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    #[serde(rename = "user_id")]
    pub user_id: i32,
    #[serde(rename = "first_name")]
    pub first_name: String,
    #[serde(rename = "email")]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: i32,      // user_id
    #[serde(rename = "email")]
    pub email: String,
    pub exp: usize,
}
