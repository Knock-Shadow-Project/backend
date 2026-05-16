//! Carga tipada de configuración para api-db.
//!
//! Centraliza la lectura de variables de entorno en un único punto, eliminando
//! `env::var(...)` esparcidos por el código y proveyendo errores explícitos
//! al arranque. Refleja el patrón fail-fast aplicado a `JWT_SECRET` en
//! `auth::init_jwt_secret` (Phase A.1).

use std::env;
use std::error::Error;
use std::fmt;

/// Configuración del servidor api-db cargada al arranque.
///
/// Todos los campos requeridos fallan rápido si la env var falta o es inválida.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
}

/// Errores de configuración con mensaje legible para humanos.
#[derive(Debug)]
pub struct ConfigError(pub String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "config error: {}", self.0)
    }
}

impl Error for ConfigError {}

impl Config {
    /// Carga la configuración desde el entorno.
    ///
    /// `DATABASE_URL` es obligatorio: sin él el servicio no debería arrancar.
    /// `PORT` es opcional (default 3000) y debe ser un u16 válido si se pasa.
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = env::var("DATABASE_URL").map_err(|_| {
            ConfigError(
                "DATABASE_URL environment variable is required \
                 (e.g. postgres://user:pass@host:5432/db)"
                    .to_string(),
            )
        })?;

        let port_raw = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let port: u16 = port_raw
            .parse()
            .map_err(|e| ConfigError(format!("PORT must be a u16 (got {port_raw:?}): {e}")))?;

        Ok(Self { database_url, port })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serializa tests que tocan env vars (otro test podría sobreescribir
    // mientras leemos).
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn from_env_requires_database_url() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { env::remove_var("DATABASE_URL") };
        let err = Config::from_env().unwrap_err();
        assert!(err.0.contains("DATABASE_URL"));
    }

    #[test]
    fn from_env_parses_port() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { env::set_var("DATABASE_URL", "postgres://x") };
        unsafe { env::set_var("PORT", "8080") };
        let cfg = Config::from_env().unwrap();
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.database_url, "postgres://x");
        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("PORT");
        }
    }

    #[test]
    fn from_env_rejects_invalid_port() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { env::set_var("DATABASE_URL", "postgres://x") };
        unsafe { env::set_var("PORT", "abc") };
        let err = Config::from_env().unwrap_err();
        assert!(err.0.contains("PORT"));
        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("PORT");
        }
    }
}
