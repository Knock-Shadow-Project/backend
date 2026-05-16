//! Loop de sincronización Pi → cloud con retry y backoff exponencial.
//!
//! Cambios clave vs. la versión anterior:
//!
//! 1. **No bloqueo de cabecera (HOL blocking)**: un fallo en un item no
//!    interrumpe el lote — antes, un único error de red devolvía `Err` y
//!    dejaba sin procesar todos los items restantes del batch.
//!
//! 2. **Backoff exponencial por item**: cada fila lleva `attempts` y
//!    `last_attempt_at`. La consulta del lote sólo selecciona filas elegibles
//!    para reintento (han pasado >= backoff(attempts) segundos desde el
//!    último intento). Esto evita que un payload "incurable" (ej. servidor
//!    devolviendo 400 por validación) consuma todos los recursos del loop.
//!
//! 3. **Tolerancia a errores transitorios**: cualquier error (red u HTTP)
//!    incrementa `attempts` y vuelve a dejar el item en la queue. El loop
//!    completa el resto del batch y duerme hasta el siguiente tick.

use sqlx::{Pool, Sqlite};
use std::time::Duration;
use tracing::{info, warn};

/// Intervalo entre lotes de sincronización.
const TICK_INTERVAL: Duration = Duration::from_secs(30);
/// Tamaño máximo de lote por tick.
const BATCH_SIZE: i64 = 50;
/// Timeout HTTP por request individual.
const HTTP_TIMEOUT: Duration = Duration::from_secs(15);
/// Backoff mínimo entre reintentos del mismo item.
const BACKOFF_BASE_SECS: i64 = 5;
/// Backoff máximo entre reintentos del mismo item.
const BACKOFF_MAX_SECS: i64 = 300;

pub async fn run(db: Pool<Sqlite>, api_base_url: String) {
    let mut interval = tokio::time::interval(TICK_INTERVAL);
    loop {
        interval.tick().await;
        if let Err(e) = sync_batch(&db, &api_base_url).await {
            // sync_batch sólo devuelve Err para fallos estructurales (ej. la
            // propia base de datos local), no por errores de red de items.
            warn!("Sync batch infrastructure failure: {}", e);
        }
    }
}

/// Calcula el backoff en segundos para un item según sus intentos previos.
///
/// Exponencial con base `BACKOFF_BASE_SECS`, cap en `BACKOFF_MAX_SECS`.
/// Nota: en producción la elegibilidad se calcula server-side dentro del SELECT
/// para evitar traer filas no-elegibles, pero mantenemos esta función como
/// referencia y para validar el cálculo en tests.
#[allow(dead_code)]
fn backoff_seconds(attempts: i64) -> i64 {
    if attempts <= 0 {
        return 0;
    }
    // 2^(attempts-1) * base, saturando para evitar overflow con muchos intentos.
    let shift = (attempts - 1).min(20) as u32;
    BACKOFF_BASE_SECS
        .saturating_mul(1i64 << shift)
        .min(BACKOFF_MAX_SECS)
}

async fn sync_batch(db: &Pool<Sqlite>, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Selecciona items elegibles para reintento ahora: primera vez (attempts=0)
    // o cuya espera (backoff) ya ha pasado. SQLite no permite expresiones de
    // INTERVAL parametrizadas con datetime() de forma trivial, así que
    // pre-calculamos el "now - backoff" en código y comparamos contra
    // `last_attempt_at`.
    let now = chrono::Utc::now();

    let items = sqlx::query_as::<_, (i64, String, String, String, Option<String>, i64)>(
        "SELECT id, method, endpoint, payload, headers, attempts
         FROM sync_queue
         WHERE last_attempt_at IS NULL
            OR (strftime('%s', 'now') - strftime('%s', last_attempt_at)) >=
               CASE
                 WHEN attempts <= 0 THEN 0
                 WHEN attempts >= 7 THEN ?1
                 ELSE ?2 * (1 << (attempts - 1))
               END
         ORDER BY id ASC
         LIMIT ?3",
    )
    .bind(BACKOFF_MAX_SECS)
    .bind(BACKOFF_BASE_SECS)
    .bind(BATCH_SIZE)
    .fetch_all(db)
    .await?;

    if items.is_empty() {
        return Ok(());
    }

    let client = reqwest::Client::builder().timeout(HTTP_TIMEOUT).build()?;

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (id, method, endpoint, payload, headers, attempts) in items {
        let url = format!("{}{}", base_url.trim_end_matches('/'), endpoint);
        let mut req = match method.as_str() {
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "PATCH" => client.patch(&url),
            _ => client.post(&url),
        };

        req = req.body(payload).header("Content-Type", "application/json");

        if let Some(ref h) = headers
            && let Ok(map) = serde_json::from_str::<serde_json::Value>(h)
            && let Some(obj) = map.as_object()
        {
            for (k, v) in obj {
                if let Some(vs) = v.as_str() {
                    req = req.header(k, vs);
                }
            }
        }

        // CLAVE: cualquier resultado (Ok/Err) progresa al siguiente item.
        // Antes, `Err(e) => return Err(...)` mataba el lote completo.
        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                if let Err(e) = sqlx::query("DELETE FROM sync_queue WHERE id = ?1")
                    .bind(id)
                    .execute(db)
                    .await
                {
                    warn!("Failed to delete synced item {}: {}", id, e);
                } else {
                    succeeded += 1;
                    info!("Synced queue item {} (after {} attempts)", id, attempts + 1);
                }
            }
            Ok(resp) => {
                let status = resp.status();
                warn!(
                    "Sync item {} got HTTP {} (attempt {}); will retry with backoff",
                    id,
                    status,
                    attempts + 1
                );
                mark_failure(db, id, &now).await;
                failed += 1;
            }
            Err(e) => {
                warn!(
                    "Sync item {} network error (attempt {}): {}; will retry",
                    id,
                    attempts + 1,
                    e
                );
                mark_failure(db, id, &now).await;
                failed += 1;
            }
        }
    }

    if succeeded > 0 || failed > 0 {
        info!("Sync batch finished: {} ok, {} failed", succeeded, failed);
    }

    Ok(())
}

/// Incrementa el contador de intentos y registra el timestamp del intento.
///
/// Se ignora cualquier error de UPDATE: si la base de datos local falla aquí,
/// el siguiente tick simplemente reintentará el item, y el error se loggeará
/// en su propio contexto.
async fn mark_failure(db: &Pool<Sqlite>, id: i64, now: &chrono::DateTime<chrono::Utc>) {
    let now_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
    if let Err(e) = sqlx::query(
        "UPDATE sync_queue
         SET attempts = attempts + 1, last_attempt_at = ?1
         WHERE id = ?2",
    )
    .bind(&now_str)
    .bind(id)
    .execute(db)
    .await
    {
        warn!(
            "Failed to record retry attempt for sync_queue id={}: {}",
            id, e
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_starts_at_zero_for_first_attempt() {
        assert_eq!(backoff_seconds(0), 0);
    }

    #[test]
    fn backoff_doubles_geometrically() {
        assert_eq!(backoff_seconds(1), BACKOFF_BASE_SECS);
        assert_eq!(backoff_seconds(2), BACKOFF_BASE_SECS * 2);
        assert_eq!(backoff_seconds(3), BACKOFF_BASE_SECS * 4);
        assert_eq!(backoff_seconds(4), BACKOFF_BASE_SECS * 8);
    }

    #[test]
    fn backoff_caps_at_max() {
        // attempts=7 → 2^6 * 5 = 320 → cap 300
        assert_eq!(backoff_seconds(7), BACKOFF_MAX_SECS);
        // Aún más altos también capean.
        assert_eq!(backoff_seconds(100), BACKOFF_MAX_SECS);
    }
}
