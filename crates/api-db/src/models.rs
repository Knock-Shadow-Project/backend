use bigdecimal::BigDecimal;
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Usuario {
    pub id_usuario: i32,
    pub nombre: String,
    pub apellido: String,
    pub correo: String,
    pub telefono: Option<String>,
    pub edad: Option<i32>,
    pub peso: Option<BigDecimal>,
    pub estatura: Option<i32>,
    pub pais: Option<String>,
    pub ciudad: Option<String>,
    pub direccion: Option<String>,
    pub lateralidad: Option<String>,
    pub nivel: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUsuario {
    pub nombre: String,
    pub apellido: String,
    pub correo: String,
    pub contrasena: String,
    pub telefono: Option<String>,
    pub edad: Option<i32>,
    pub peso: Option<BigDecimal>,
    pub estatura: Option<i32>,
    pub pais: Option<String>,
    pub ciudad: Option<String>,
    pub direccion: Option<String>,
    pub lateralidad: Option<String>,
    pub nivel: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUsuario {
    pub nombre: Option<String>,
    pub apellido: Option<String>,
    pub correo: Option<String>,
    pub contrasena: Option<String>,
    pub telefono: Option<String>,
    pub edad: Option<i32>,
    pub peso: Option<BigDecimal>,
    pub estatura: Option<i32>,
    pub pais: Option<String>,
    pub ciudad: Option<String>,
    pub direccion: Option<String>,
    pub lateralidad: Option<String>,
    pub nivel: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Entrenamiento {
    pub id_entrenamiento: i32,
    pub hora_inicio: NaiveDateTime,
    pub hora_fin: Option<NaiveDateTime>,
    pub tipo: Option<String>,
    pub calorias: Option<i32>,
    pub id_usuario: i32,
}

#[derive(Debug, Deserialize)]
pub struct CreateEntrenamiento {
    pub hora_inicio: NaiveDateTime,
    pub hora_fin: Option<NaiveDateTime>,
    pub tipo: Option<String>,
    pub calorias: Option<i32>,
    pub id_usuario: i32,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEntrenamiento {
    pub hora_inicio: Option<NaiveDateTime>,
    pub hora_fin: Option<NaiveDateTime>,
    pub tipo: Option<String>,
    pub calorias: Option<i32>,
    pub id_usuario: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Golpe {
    pub id_golpe: i32,
    pub nombre: String,
    pub extremidad: Option<String>,
    pub posicion: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGolpe {
    pub nombre: String,
    pub extremidad: Option<String>,
    pub posicion: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateGolpe {
    pub nombre: Option<String>,
    pub extremidad: Option<String>,
    pub posicion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Historial {
    pub id_entrenamiento: i32,
    pub id_golpe: i32,
    pub potencia: Option<BigDecimal>,
}

#[derive(Debug, Deserialize)]
pub struct CreateHistorial {
    pub id_entrenamiento: i32,
    pub id_golpe: i32,
    pub potencia: Option<BigDecimal>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateHistorial {
    pub potencia: Option<BigDecimal>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct HistorialDetail {
    pub id_entrenamiento: i32,
    pub id_golpe: i32,
    pub potencia: Option<BigDecimal>,
    pub nombre: String,
    pub extremidad: Option<String>,
    pub posicion: Option<String>,
}

// Auth models ---------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub correo: String,
    pub contrasena: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub id_usuario: i32,
    pub nombre: String,
    pub correo: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: i32,      // id_usuario
    pub email: String,
    pub exp: usize,
}
