//! Carga tipada de configuración para api-db.
//!
//! Centraliza la lectura de variables de entorno en un único punto, eliminando
//! `env::var(...)` esparcidos por el código y proveyendo errores explícitos
//! al arranque. Refleja el patrón fail-fast aplicado a `JWT_SECRET` en
//! `auth::init_jwt_secret` (Phase A.1).

use std::env;
use std::error::Error;
use std::fmt;

/// Sender por defecto usado si no se especifica `RESEND_FROM`. Apunta a
/// un buzón "no-reply" del dominio que se haya verificado en Resend.
const DEFAULT_FROM: &str = "KnockShadow <no-reply@knockshadow.site>";

/// Base URL por defecto que se usa al construir el enlace de confirmación
/// del email. Apunta al subdominio público que ya está en producción
/// (`api.knockshadow.site`). Si más adelante se separa un host dedicado
/// para la pantalla de confirmación, basta con sobreescribirlo vía
/// `APP_BASE_URL` en el entorno del servicio.
const DEFAULT_APP_BASE_URL: &str = "https://api.knockshadow.site";

/// Configuración del servidor api-db cargada al arranque.
///
/// Todos los campos requeridos fallan rápido si la env var falta o es inválida.
/// Los campos opcionales (email) sólo se loggean si no están presentes.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub port: u16,

    /// API key de Resend. Si es `None`, el envío de emails queda
    /// desactivado y los endpoints que lo necesiten caen al modo stub
    /// (loggear en vez de enviar). Permite arrancar el servidor en
    /// entornos de CI/dev sin tener que pinchar Resend de verdad.
    pub resend_api_key: Option<String>,

    /// Dirección "From" usada al enviar emails. Debe ser un buzón del
    /// dominio verificado en Resend o el envío fallará con 403.
    pub resend_from: String,

    /// Base URL del frontend. Se usa para componer los enlaces que se
    /// envían por email (ej. `{app_base_url}/confirm-email?token=...`).
    pub app_base_url: String,

    /// URL de Redis para el rate-limit distribuido (`redis://host:6379/`).
    /// Si es `None`, el rate-limit cae al backend in-memory (suficiente
    /// para 1 pod). Cuando arranques múltiples instancias detrás de un
    /// balanceador, define esta var para que el throttle sea global.
    pub redis_url: Option<String>,
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
    /// `RESEND_API_KEY`, `RESEND_FROM`, `APP_BASE_URL` son opcionales con
    /// fallbacks razonables; sin la API key el envío real queda desactivado.
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

        // Aceptamos `RESEND_API_KEY` ausente o vacío como "sin email": en
        // ambos casos el handler caerá al stub. Esto evita arranques
        // inesperados si alguien define la var pero la deja en blanco.
        let resend_api_key = env::var("RESEND_API_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let resend_from = env::var("RESEND_FROM").unwrap_or_else(|_| DEFAULT_FROM.to_string());

        let app_base_url =
            env::var("APP_BASE_URL").unwrap_or_else(|_| DEFAULT_APP_BASE_URL.to_string());

        // Mismo criterio que `RESEND_API_KEY`: ausente o cadena vacía
        // cuentan como "no Redis". Evita arranques con URL en blanco que
        // confundirían los logs.
        let redis_url = env::var("REDIS_URL")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        Ok(Self {
            database_url,
            port,
            resend_api_key,
            resend_from,
            app_base_url,
            redis_url,
        })
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

    #[test]
    fn from_env_email_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { env::set_var("DATABASE_URL", "postgres://x") };
        unsafe { env::remove_var("RESEND_API_KEY") };
        unsafe { env::remove_var("RESEND_FROM") };
        unsafe { env::remove_var("APP_BASE_URL") };
        let cfg = Config::from_env().unwrap();
        assert!(cfg.resend_api_key.is_none());
        assert_eq!(cfg.resend_from, DEFAULT_FROM);
        assert_eq!(cfg.app_base_url, DEFAULT_APP_BASE_URL);
        unsafe { env::remove_var("DATABASE_URL") };
    }

    #[test]
    fn from_env_treats_empty_api_key_as_unset() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { env::set_var("DATABASE_URL", "postgres://x") };
        unsafe { env::set_var("RESEND_API_KEY", "   ") };
        let cfg = Config::from_env().unwrap();
        assert!(cfg.resend_api_key.is_none());
        unsafe {
            env::remove_var("DATABASE_URL");
            env::remove_var("RESEND_API_KEY");
        }
    }
}
