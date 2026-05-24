use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
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
    #[schema(value_type = Option<f64>)]
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
    #[serde(rename = "confirmed")]
    #[sqlx(rename = "confirmado")]
    pub confirmed: bool,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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
    #[schema(value_type = Option<f64>)]
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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
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
    #[schema(value_type = Option<f64>)]
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

#[derive(Debug, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Entrenamiento {
    #[serde(rename = "training_id")]
    #[sqlx(rename = "id_entrenamiento")]
    pub training_id: i32,
    #[serde(rename = "user_id")]
    #[sqlx(rename = "id_usuario")]
    pub user_id: i32,
    #[serde(rename = "routine_id")]
    #[sqlx(rename = "id_rutina")]
    pub routine_id: Option<i32>,
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
    #[serde(rename = "current_step")]
    #[sqlx(rename = "paso_actual")]
    pub current_step: Option<i32>,
    #[serde(rename = "state")]
    #[sqlx(rename = "estado")]
    pub state: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateEntrenamiento {
    #[serde(rename = "user_id")]
    pub user_id: i32,
    #[serde(rename = "routine_id")]
    pub routine_id: Option<i32>,
    #[serde(rename = "start_time")]
    pub start_time: Option<NaiveDateTime>,
    #[serde(rename = "end_time")]
    pub end_time: Option<NaiveDateTime>,
    #[serde(rename = "training_type")]
    pub training_type: Option<String>,
    #[serde(rename = "calories")]
    pub calories: Option<i32>,
    #[serde(rename = "current_step")]
    pub current_step: Option<i32>,
    #[serde(rename = "state")]
    pub state: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateEntrenamiento {
    #[serde(rename = "user_id")]
    pub user_id: Option<i32>,
    #[serde(rename = "routine_id")]
    pub routine_id: Option<i32>,
    #[serde(rename = "start_time")]
    pub start_time: Option<NaiveDateTime>,
    #[serde(rename = "end_time")]
    pub end_time: Option<NaiveDateTime>,
    #[serde(rename = "training_type")]
    pub training_type: Option<String>,
    #[serde(rename = "calories")]
    pub calories: Option<i32>,
    #[serde(rename = "current_step")]
    pub current_step: Option<i32>,
    #[serde(rename = "state")]
    pub state: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
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

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateGolpe {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "limb")]
    pub limb: String,
    #[serde(rename = "position")]
    pub position: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateGolpe {
    #[serde(rename = "name")]
    pub name: Option<String>,
    #[serde(rename = "limb")]
    pub limb: Option<String>,
    #[serde(rename = "position")]
    pub position: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Rutina {
    #[serde(rename = "routine_id")]
    #[sqlx(rename = "id_rutina")]
    pub routine_id: i32,
    #[serde(rename = "name")]
    #[sqlx(rename = "nombre")]
    pub name: String,
    #[serde(rename = "recommended_level")]
    #[sqlx(rename = "nivel_recomendado")]
    pub recommended_level: Option<String>,
    #[serde(rename = "punch_sequence")]
    #[sqlx(rename = "secuencia_golpes")]
    pub punch_sequence: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateRutina {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "recommended_level")]
    pub recommended_level: Option<String>,
    #[serde(rename = "punch_sequence")]
    pub punch_sequence: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateRutina {
    #[serde(rename = "name")]
    pub name: Option<String>,
    #[serde(rename = "recommended_level")]
    pub recommended_level: Option<String>,
    #[serde(rename = "punch_sequence")]
    pub punch_sequence: Option<Vec<i32>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct Historial {
    #[serde(rename = "history_id")]
    #[sqlx(rename = "id_historial")]
    pub history_id: i32,
    #[serde(rename = "training_id")]
    #[sqlx(rename = "id_entrenamiento")]
    pub training_id: i32,
    #[serde(rename = "thrown_punch_id")]
    #[sqlx(rename = "id_golpe_lanzado")]
    pub thrown_punch_id: i32,
    #[serde(rename = "expected_punch_id")]
    #[sqlx(rename = "id_golpe_esperado")]
    pub expected_punch_id: Option<i32>,
    #[serde(rename = "power")]
    #[sqlx(rename = "potencia")]
    #[schema(value_type = Option<f64>)]
    pub power: Option<BigDecimal>,
    #[serde(rename = "is_correct")]
    #[sqlx(rename = "es_correcto")]
    pub is_correct: Option<bool>,
    #[serde(rename = "impact_date")]
    #[sqlx(rename = "fecha_impacto")]
    pub impact_date: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateHistorial {
    #[serde(rename = "training_id")]
    pub training_id: i32,
    #[serde(rename = "thrown_punch_id")]
    pub thrown_punch_id: i32,
    #[serde(rename = "expected_punch_id")]
    pub expected_punch_id: Option<i32>,
    #[serde(rename = "power")]
    #[schema(value_type = Option<f64>)]
    pub power: Option<BigDecimal>,
    #[serde(rename = "is_correct")]
    pub is_correct: Option<bool>,
    #[serde(rename = "impact_date")]
    pub impact_date: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateHistorial {
    #[serde(rename = "thrown_punch_id")]
    pub thrown_punch_id: Option<i32>,
    #[serde(rename = "expected_punch_id")]
    pub expected_punch_id: Option<i32>,
    #[serde(rename = "power")]
    #[schema(value_type = Option<f64>)]
    pub power: Option<BigDecimal>,
    #[serde(rename = "is_correct")]
    pub is_correct: Option<bool>,
    #[serde(rename = "impact_date")]
    pub impact_date: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct HistorialDetail {
    #[serde(rename = "history_id")]
    #[sqlx(rename = "id_historial")]
    pub history_id: i32,
    #[serde(rename = "training_id")]
    #[sqlx(rename = "id_entrenamiento")]
    pub training_id: i32,
    #[serde(rename = "thrown_punch_id")]
    #[sqlx(rename = "id_golpe_lanzado")]
    pub thrown_punch_id: i32,
    #[serde(rename = "expected_punch_id")]
    #[sqlx(rename = "id_golpe_esperado")]
    pub expected_punch_id: Option<i32>,
    #[serde(rename = "power")]
    #[sqlx(rename = "potencia")]
    #[schema(value_type = Option<f64>)]
    pub power: Option<BigDecimal>,
    #[serde(rename = "is_correct")]
    #[sqlx(rename = "es_correcto")]
    pub is_correct: Option<bool>,
    #[serde(rename = "impact_date")]
    #[sqlx(rename = "fecha_impacto")]
    pub impact_date: Option<NaiveDateTime>,
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

#[derive(Debug, Serialize, Deserialize, FromRow, utoipa::ToSchema)]
pub struct ResultadoFuerza {
    #[serde(rename = "result_id")]
    #[sqlx(rename = "id_resultado")]
    pub result_id: i32,
    #[serde(rename = "user_id")]
    #[sqlx(rename = "id_usuario")]
    pub user_id: Option<i32>,
    #[serde(rename = "participant_name")]
    #[sqlx(rename = "nombre_participante")]
    pub participant_name: String,
    #[serde(rename = "score")]
    #[sqlx(rename = "puntuacion")]
    pub score: i32,
    #[serde(rename = "mode")]
    #[sqlx(rename = "modo")]
    pub mode: String,
    #[serde(rename = "group_id")]
    #[sqlx(rename = "grupo")]
    pub group_id: Option<String>,
    #[serde(rename = "date")]
    #[sqlx(rename = "fecha")]
    pub date: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateResultadoFuerza {
    #[serde(rename = "user_id")]
    pub user_id: Option<i32>,
    #[serde(rename = "participant_name")]
    pub participant_name: String,
    #[serde(rename = "score")]
    pub score: i32,
    #[serde(rename = "mode")]
    pub mode: Option<String>,
    #[serde(rename = "group_id")]
    pub group_id: Option<String>,
}

pub const DEFAULT_PAGE_SIZE: i64 = 50;
pub const MAX_PAGE_SIZE: i64 = 200;

#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct Pagination {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

impl Pagination {
    pub fn resolved_limit(&self) -> i64 {
        self.limit
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE)
    }

    pub fn resolved_offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    #[serde(rename = "email")]
    pub email: String,
    #[serde(rename = "password")]
    pub password: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub token: String,
    #[serde(rename = "user_id")]
    pub user_id: i32,
    #[serde(rename = "first_name")]
    pub first_name: String,
    #[serde(rename = "email")]
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TokenClaims {
    pub sub: i32,
    #[serde(rename = "email")]
    pub email: String,
    pub exp: usize,
}
