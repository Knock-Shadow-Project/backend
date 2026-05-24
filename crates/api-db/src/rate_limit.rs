//! Rate-limit abstraction con dos backends conmutables en tiempo de
//! arranque: `InMemory` (default) y `Redis` (cuando se proporciona
//! `REDIS_URL`).
//!
//! > **Sobre el nombre.** Internamente la variante se llama `Redis` y la
//! > env var es `REDIS_URL` porque el crate cliente y la convención de
//! > la industria así lo nombran (`redis://...`). En realidad el
//! > servidor que usamos es **Valkey** (fork BSD-3 mantenido por la
//! > Linux Foundation). Valkey habla RESP, así que el crate `redis` se
//! > conecta y opera sin cambios.
//!
//! Diseño:
//!
//! - **Enum dispatch** en vez de `dyn RateLimiter`. Sale más simple de
//!   leer y evita arrastrar `async_trait` solo por dos variantes.
//! - **Fail-open** ante caída del backend remoto: si el comando da error
//!   devolvemos `true` (consume el cupo) para no bloquear el flujo del
//!   usuario por un blip de infra. El error se loggea con `tracing::error!`
//!   para que se vea en Grafana/Loki.
//! - Cada `try_acquire` es **un único round-trip** (`SET key value NX EX
//!   secs`) — atómico, no necesita scripts Lua.
//!
//! Uso:
//!
//! ```ignore
//! let rl = RateLimiter::from_url(cfg.redis_url.as_deref()).await;
//! if !rl.try_acquire("resend:user@x.com", 60).await {
//!     return Err(StatusCode::TOO_MANY_REQUESTS);
//! }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use redis::aio::ConnectionManager;

/// Timeout total para la conexión inicial a Redis/Valkey. Si pasa este
/// tiempo sin conectar, fallamos rápido y caemos al backend in-memory.
/// Sin esto, `ConnectionManager::new` reintenta indefinidamente cuando
/// no hay nada al otro lado (vimos tests tardar >7min contra puerto cerrado).
const REDIS_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

/// Backend de rate-limit elegido al arranque. `Clone` para poder meterlo
/// en `AppState` (cada request clona el state via extractor).
///
/// Ambas variantes envuelven el estado pesado en `Arc` para que (a) la
/// clonación sea un refcount bump y (b) el tamaño del enum no se infle
/// con la variante Redis (`ConnectionManager` son ~240 bytes; sin `Arc`
/// disparaba `clippy::large_enum_variant`).
#[derive(Clone)]
pub enum RateLimiter {
    InMemory(Arc<Mutex<HashMap<String, Instant>>>),
    Redis(Arc<ConnectionManager>),
}

impl std::fmt::Debug for RateLimiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InMemory(_) => f.write_str("RateLimiter::InMemory"),
            // No volcamos la URL ni el state del ConnectionManager: no
            // queremos credenciales en logs accidentales.
            Self::Redis(_) => f.write_str("RateLimiter::Redis"),
        }
    }
}

impl RateLimiter {
    /// Crea el backend in-memory. Útil tanto como fallback de Redis como
    /// para tests unitarios que no quieren tocar infra externa.
    pub fn in_memory() -> Self {
        Self::InMemory(Arc::new(Mutex::new(HashMap::new())))
    }

    /// Intenta conectar a Redis/Valkey. Devuelve `Err` si la URL no
    /// resuelve, el handshake inicial falla o no responde dentro de
    /// `REDIS_CONNECT_TIMEOUT`. El caller decide si cae a in-memory o
    /// aborta el arranque.
    pub async fn redis(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        // `ConnectionManager::new` reintenta backoff exponencial sin techo
        // si la conexión inicial falla. Lo envolvemos en un timeout duro
        // para fail-fast en tests/arranques donde el backend no existe.
        let conn = match tokio::time::timeout(REDIS_CONNECT_TIMEOUT, ConnectionManager::new(client))
            .await
        {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => return Err(e),
            Err(_elapsed) => {
                // Sintetizamos un error que conserva el formato que
                // espera `RedisResult`. La causa real (timeout) queda
                // explícita en el mensaje.
                return Err(redis::RedisError::from(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    format!(
                        "Redis connect timeout tras {}s",
                        REDIS_CONNECT_TIMEOUT.as_secs()
                    ),
                )));
            }
        };
        Ok(Self::Redis(Arc::new(conn)))
    }

    /// Construye el backend a partir de la `REDIS_URL` opcional. Si no se
    /// proporciona, devuelve `InMemory`. Si se proporciona pero Redis está
    /// caído, también devuelve `InMemory` con un warning — preferimos
    /// arrancar degradado a no arrancar.
    pub async fn from_url(url: Option<&str>) -> Self {
        match url {
            Some(u) if !u.is_empty() => match Self::redis(u).await {
                Ok(rl) => {
                    tracing::info!("rate_limit: backend Redis activo");
                    rl
                }
                Err(e) => {
                    tracing::warn!(
                        "rate_limit: no se pudo conectar a Redis ({}), cae a memoria",
                        e
                    );
                    Self::in_memory()
                }
            },
            _ => {
                tracing::info!("rate_limit: backend en memoria (REDIS_URL no definido)");
                Self::in_memory()
            }
        }
    }

    /// Devuelve `true` si la clave estaba libre (cupo consumido para los
    /// próximos `window_secs` segundos), `false` si todavía estaba dentro
    /// de la ventana de bloqueo.
    ///
    /// Fail-open: si el backend remoto falla devolvemos `true`. El email
    /// se mandará aunque la infra de rate-limit esté KO, lo que es mejor
    /// que bloquear a todo el mundo por un blip.
    pub async fn try_acquire(&self, key: &str, window_secs: u64) -> bool {
        match self {
            Self::InMemory(map) => {
                let mut guard = map.lock().expect("rate-limit map poisoned");
                let now = Instant::now();
                if let Some(last) = guard.get(key)
                    && now.duration_since(*last) < Duration::from_secs(window_secs)
                {
                    return false;
                }
                guard.insert(key.to_string(), now);
                true
            }
            Self::Redis(conn_arc) => {
                // `ConnectionManager` mantiene un pool interno; clonarlo es
                // un Arc-bump del estado compartido (cheap). Necesitamos
                // ownership porque `query_async` toma `&mut` y no podemos
                // tomar &mut a través del Arc.
                let mut conn = ConnectionManager::clone(conn_arc.as_ref());
                // SET <key> 1 NX EX <window_secs>:
                //   - NX: sólo si no existe ya  → atómico
                //   - EX: TTL en segundos       → cleanup automático
                // Respuesta: "OK" si se setteó (acquired) o nil si ya existía.
                let result: redis::RedisResult<Option<String>> = redis::cmd("SET")
                    .arg(key)
                    .arg(1)
                    .arg("NX")
                    .arg("EX")
                    .arg(window_secs)
                    .query_async(&mut conn)
                    .await;
                match result {
                    Ok(Some(_)) => true,
                    Ok(None) => false,
                    Err(e) => {
                        tracing::error!(
                            "rate_limit: Redis SET falló ({}), fail-open (permitiendo)",
                            e
                        );
                        true
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn in_memory_blocks_within_window() {
        let rl = RateLimiter::in_memory();
        assert!(rl.try_acquire("k1", 60).await);
        // Segunda llamada inmediata: bloqueada.
        assert!(!rl.try_acquire("k1", 60).await);
        // Clave distinta: libre.
        assert!(rl.try_acquire("k2", 60).await);
    }

    #[tokio::test]
    async fn in_memory_releases_after_window() {
        let rl = RateLimiter::in_memory();
        assert!(rl.try_acquire("k1", 0).await);
        // Con window=0 el siguiente intento ya está fuera de ventana.
        // Le metemos un pequeño sleep para que `Instant::now()` avance.
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(rl.try_acquire("k1", 0).await);
    }

    #[tokio::test]
    async fn redis_url_missing_falls_back_to_memory() {
        let rl = RateLimiter::from_url(None).await;
        match rl {
            RateLimiter::InMemory(_) => {}
            _ => panic!("esperaba InMemory cuando REDIS_URL es None"),
        }
    }

    #[tokio::test]
    async fn redis_url_empty_falls_back_to_memory() {
        let rl = RateLimiter::from_url(Some("")).await;
        match rl {
            RateLimiter::InMemory(_) => {}
            _ => panic!("esperaba InMemory cuando REDIS_URL es cadena vacía"),
        }
    }

    #[tokio::test]
    async fn redis_url_unreachable_falls_back_to_memory() {
        // Puerto cerrado garantizado en localhost (no usado por servicios
        // típicos). Si esto empieza a fallar en CI, cámbialo por 1 (que
        // siempre devuelve "permission denied").
        let rl = RateLimiter::from_url(Some("redis://127.0.0.1:1/")).await;
        match rl {
            RateLimiter::InMemory(_) => {}
            _ => panic!("esperaba fallback a InMemory cuando Redis no responde"),
        }
    }
}
