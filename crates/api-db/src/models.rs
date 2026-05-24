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
    /// Estado de confirmación de email. Lo añade la migración 003 con
    /// DEFAULT FALSE; los handlers de `/register` no lo escriben (sale en
    /// FALSE) y sólo `GET /confirm-email` lo flippa a TRUE. Se serializa
    /// como `confirmed` para mantener la convención inglesa del JSON.
    #[serde(rename = "confirmed")]
    #[sqlx(rename = "confirmado")]
    pub confirmed: bool,
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
    #[serde(rename = "user_id")]
    #[sqlx(rename = "id_usuario")]
    pub user_id: i32,
    /// FK opcional a RUTINA. `None` cuando el entrenamiento es de tipo 'Libre'.
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
    /// Paso actual de la rutina guiada (cursor sobre `SECUENCIA_GOLPES`).
    /// Se mantiene en 0 para entrenamientos 'Libre'.
    #[serde(rename = "current_step")]
    #[sqlx(rename = "paso_actual")]
    pub current_step: Option<i32>,
    /// Estado del entrenamiento: ACTIVO, PAUSADO, FINALIZADO…
    #[serde(rename = "state")]
    #[sqlx(rename = "estado")]
    pub state: Option<String>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Golpe {
    #[serde(rename = "punch_id")]
    #[sqlx(rename = "id_golpe")]
    pub punch_id: i32,
    #[serde(rename = "name")]
    #[sqlx(rename = "nombre")]
    pub name: String,
    /// Tras la migración 002 estas columnas son NOT NULL en BD, pero las
    /// mantenemos `Option` en el modelo de salida para tolerar filas
    /// históricas que pudieran haber quedado nulas en entornos viejos.
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
    /// La migración 002 hace NOT NULL `extremidad` y `posicion`. El payload
    /// los recibe ya obligatorios; un cliente que envíe `null` fallará al
    /// deserializar, evitando insertar filas inconsistentes.
    #[serde(rename = "limb")]
    pub limb: String,
    #[serde(rename = "position")]
    pub position: String,
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

// Rutina --------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, FromRow)]
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
    /// Lista ordenada de IDs de GOLPE que definen el ritmo de la rutina.
    /// Se persiste como `INTEGER[]` en PostgreSQL; sqlx la mapea directo
    /// a `Vec<i32>`.
    #[serde(rename = "punch_sequence")]
    #[sqlx(rename = "secuencia_golpes")]
    pub punch_sequence: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRutina {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "recommended_level")]
    pub recommended_level: Option<String>,
    #[serde(rename = "punch_sequence")]
    pub punch_sequence: Option<Vec<i32>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRutina {
    #[serde(rename = "name")]
    pub name: Option<String>,
    #[serde(rename = "recommended_level")]
    pub recommended_level: Option<String>,
    #[serde(rename = "punch_sequence")]
    pub punch_sequence: Option<Vec<i32>>,
}

// Historial -----------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Historial {
    /// PK propia tras la migración 002. Permite identificar de forma única
    /// cada impacto registrado, incluso si en el mismo entrenamiento se
    /// repite el mismo golpe varias veces.
    #[serde(rename = "history_id")]
    #[sqlx(rename = "id_historial")]
    pub history_id: i32,
    #[serde(rename = "training_id")]
    #[sqlx(rename = "id_entrenamiento")]
    pub training_id: i32,
    /// Golpe efectivamente detectado por el sensor.
    #[serde(rename = "thrown_punch_id")]
    #[sqlx(rename = "id_golpe_lanzado")]
    pub thrown_punch_id: i32,
    /// Golpe que la rutina esperaba en ese paso. `None` para entrenamientos
    /// 'Libre' (sin secuencia guiada).
    #[serde(rename = "expected_punch_id")]
    #[sqlx(rename = "id_golpe_esperado")]
    pub expected_punch_id: Option<i32>,
    #[serde(rename = "power")]
    #[sqlx(rename = "potencia")]
    pub power: Option<BigDecimal>,
    /// Indica si el golpe lanzado coincide con el esperado. Por defecto
    /// TRUE (modo 'Libre' o cuando el cliente no calcula la comparación).
    #[serde(rename = "is_correct")]
    #[sqlx(rename = "es_correcto")]
    pub is_correct: Option<bool>,
    #[serde(rename = "impact_date")]
    #[sqlx(rename = "fecha_impacto")]
    pub impact_date: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct CreateHistorial {
    #[serde(rename = "training_id")]
    pub training_id: i32,
    #[serde(rename = "thrown_punch_id")]
    pub thrown_punch_id: i32,
    #[serde(rename = "expected_punch_id")]
    pub expected_punch_id: Option<i32>,
    #[serde(rename = "power")]
    pub power: Option<BigDecimal>,
    #[serde(rename = "is_correct")]
    pub is_correct: Option<bool>,
    /// Si el cliente no lo manda, la BD aplica `CURRENT_TIMESTAMP`.
    #[serde(rename = "impact_date")]
    pub impact_date: Option<NaiveDateTime>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHistorial {
    #[serde(rename = "thrown_punch_id")]
    pub thrown_punch_id: Option<i32>,
    #[serde(rename = "expected_punch_id")]
    pub expected_punch_id: Option<i32>,
    #[serde(rename = "power")]
    pub power: Option<BigDecimal>,
    #[serde(rename = "is_correct")]
    pub is_correct: Option<bool>,
    #[serde(rename = "impact_date")]
    pub impact_date: Option<NaiveDateTime>,
}

/// Proyección enriquecida del historial: incluye los datos del golpe lanzado
/// (catálogo `GOLPE`) para que los clientes no tengan que cruzar manualmente.
#[derive(Debug, Serialize, FromRow)]
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

// Pagination -----------------------------------------------------------------

/// Pagination defaults applied across list endpoints.
///
/// 50 strikes a balance between mobile clients (smaller payloads, snappier
/// scroll) and admin tooling (need enough rows on a page). The 200 cap prevents
/// `?limit=10000000` from turning into a memory exhaustion vector.
pub const DEFAULT_PAGE_SIZE: i64 = 50;
pub const MAX_PAGE_SIZE: i64 = 200;

/// Query parameters parsed by all paginated list endpoints (`?limit=…&offset=…`).
#[derive(Debug, Deserialize)]
pub struct Pagination {
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

impl Pagination {
    /// Returns the limit clamped to `[1, MAX_PAGE_SIZE]` with `DEFAULT_PAGE_SIZE` as fallback.
    pub fn resolved_limit(&self) -> i64 {
        self.limit
            .unwrap_or(DEFAULT_PAGE_SIZE)
            .clamp(1, MAX_PAGE_SIZE)
    }

    /// Returns the offset clamped to `>= 0`, defaulting to 0.
    pub fn resolved_offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
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
    pub sub: i32, // user_id
    #[serde(rename = "email")]
    pub email: String,
    pub exp: usize,
}
