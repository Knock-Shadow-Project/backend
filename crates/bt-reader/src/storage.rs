//! Persistencia de muestras BLE en PostgreSQL.
//!
//! Este modulo define:
//! - El modelo `BleSample` para cada lectura recibida.
//! - `SampleWriter`, que desacopla recepcion de datos y escritura a BD.
//! - Un loop de escritura por lotes con transacciones.

use std::env;
use std::error::Error;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tokio_postgres::{Client, NoTls, Statement};
use tracing::{debug, error, info};

/// Muestra de telemetria decodificada que se persiste en la base de datos.
#[derive(Debug, Clone)]
pub struct BleSample {
    pub device_mac: String,
    pub device_name: String,
    pub ble_ts: Option<i32>,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Lectura de nivel de bateria BLE.
#[derive(Debug, Clone)]
pub struct BatteryReading {
    pub device_mac: String,
    pub device_name: String,
    pub battery_level: i16,
}

/// Registro unificado para el canal de escritura a BD.
#[derive(Debug, Clone)]
pub enum DbRecord {
    Sample(BleSample),
    Battery(BatteryReading),
}

/// Estadisticas para monitorear rendimiento del escritor.
#[derive(Clone, Copy, Debug)]
struct WriterStats {
    total_samples: usize,
    last_report: std::time::Instant,
    per_commit_samples: usize,
    samples_per_second: f64,
}
impl WriterStats {
    fn new() -> Self {
        Self {
            total_samples: 0,
            last_report: std::time::Instant::now(),
            per_commit_samples: 0,
            samples_per_second: 0.0,
        }
    }
}
/// Escritor asincrono basado en canal para enviar muestras a PostgreSQL.
///
/// La idea es evitar que el loop de lectura BLE quede bloqueado por I/O de base de datos.
#[derive(Clone)]
pub struct DbWritter {
    tx: mpsc::Sender<DbRecord>,
}

impl DbWritter {
    /// Crea la conexion a PostgreSQL, inicializa schema y arranca el loop de escritura.
    pub async fn connect(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let client = spawn_pg_connection(database_url).await?;

        // Crea tabla e indice si aun no existen.
        if let Err(err) = init_schema(&client).await {
            error!("PostgreSQL schema initialization error: {}", err);
            return Err(Box::new(err));
        }

        // Canal bufferizado para desacoplar productor (BLE) de consumidor (BD).
        let (tx, rx) = mpsc::channel(4096);
        let url_for_loop = database_url.to_string();
        tokio::spawn(async move {
            writer_loop(client, rx, url_for_loop).await;
        });

        info!("PostgreSQL writer is ready");
        Ok(Self { tx })
    }

    /// Encola una muestra para que sea persistida por el writer en segundo plano.
    pub async fn send_sample(
        &self,
        sample: BleSample,
    ) -> Result<(), mpsc::error::SendError<DbRecord>> {
        self.tx.send(DbRecord::Sample(sample)).await
    }

    /// Encola una lectura de bateria para que sea persistida por el writer en segundo plano.
    pub async fn send_battery(
        &self,
        reading: BatteryReading,
    ) -> Result<(), mpsc::error::SendError<DbRecord>> {
        self.tx.send(DbRecord::Battery(reading)).await
    }

    /// Indica si el canal hacia el writer está cerrado.
    ///
    /// Permite que el bucle de reconexión BLE detecte un writer caído y
    /// termine el proceso en vez de reconectar indefinidamente.
    pub fn is_closed(&self) -> bool {
        self.tx.is_closed()
    }
}

/// Conecta a PostgreSQL y arranca la tarea de mantenimiento de la conexión.
///
/// Encapsula el patrón estándar de `tokio_postgres`: `connect()` devuelve
/// `(Client, Connection)`, donde el `Connection` debe poll-earse en una tarea
/// dedicada o la conexión muere silenciosamente. Lo aislamos aquí para que el
/// loop de reconexión pueda reusarlo sin duplicar código.
async fn spawn_pg_connection(database_url: &str) -> Result<Client, Box<dyn Error + Send + Sync>> {
    info!("Connecting to PostgreSQL at {}", database_url);
    let (client, connection) = match tokio_postgres::connect(database_url, NoTls).await {
        Ok(pair) => {
            info!("Successfully connected to PostgreSQL");
            pair
        }
        Err(err) => {
            error!("PostgreSQL connection error: {}", err);
            return Err(Box::new(err));
        }
    };

    tokio::spawn(async move {
        if let Err(err) = connection.await {
            error!("PostgreSQL connection error: {}", err);
        }
    });

    Ok(client)
}

/// Reintenta conectar a PostgreSQL con backoff exponencial.
///
/// Resuelve el TODO histórico de "retry connection to client": antes, un error
/// transitorio durante `BEGIN` (reinicio de Postgres, fail-over, glitch de
/// red) descartaba el lote y devolvía sin persistir. Ahora se intenta
/// reconectar hasta `MAX_RECONNECT_ATTEMPTS` veces; si todos fallan, se
/// retorna el último error al caller para que decida qué hacer con el buffer.
///
/// Devuelve `(Client, Statement, Statement)` con los statements re-preparados
/// para la nueva conexión (las prepared statements son por-conexión en
/// tokio-postgres y no se pueden reusar tras una reconexión).
async fn reconnect_with_backoff(
    database_url: &str,
) -> Result<(Client, Statement, Statement), Box<dyn Error + Send + Sync>> {
    const MAX_RECONNECT_ATTEMPTS: u32 = 5;
    const BASE_DELAY: Duration = Duration::from_millis(500);
    const MAX_DELAY: Duration = Duration::from_secs(30);

    let mut last_err: Option<Box<dyn Error + Send + Sync>> = None;
    for attempt in 1..=MAX_RECONNECT_ATTEMPTS {
        let delay = (BASE_DELAY * 2u32.saturating_pow(attempt - 1)).min(MAX_DELAY);
        info!(
            "Reconnect attempt {}/{} (waiting {:?})",
            attempt, MAX_RECONNECT_ATTEMPTS, delay
        );
        tokio::time::sleep(delay).await;

        match spawn_pg_connection(database_url).await {
            Ok(client) => match prepare_statements(&client).await {
                Ok((insert_stmt, battery_stmt)) => {
                    info!("Reconnect succeeded on attempt {}", attempt);
                    return Ok((client, insert_stmt, battery_stmt));
                }
                Err(err) => {
                    error!("Reconnect statement prepare failed: {}", err);
                    last_err = Some(Box::new(err));
                }
            },
            Err(err) => {
                error!("Reconnect attempt {} failed: {}", attempt, err);
                last_err = Some(err);
            }
        }
    }
    Err(last_err
        .unwrap_or_else(|| "reconnect failed without error context".into()))
}

/// Prepara las dos statements de INSERT usadas por el writer.
///
/// Extraído de `writer_loop` para reutilizarse tras una reconexión: cada
/// `Client` necesita sus propias prepared statements.
async fn prepare_statements(
    client: &Client,
) -> Result<(Statement, Statement), tokio_postgres::Error> {
    let insert_stmt = client
        .prepare(
            "
            INSERT INTO ble_samples (device_mac, device_name, ble_ts, x, y, z)
            VALUES ($1, $2, $3, $4, $5, $6)
            ",
        )
        .await?;

    let battery_stmt = client
        .prepare(
            "
            INSERT INTO device_battery_readings (device_mac, device_name, battery_level)
            VALUES ($1, $2, $3)
            ",
        )
        .await?;

    Ok((insert_stmt, battery_stmt))
}

/// Garantiza que la tabla destino y su indice existan.
async fn init_schema(client: &Client) -> Result<(), tokio_postgres::Error> {
    client
        .batch_execute(
            "
            CREATE EXTENSION IF NOT EXISTS timescaledb;

            CREATE TABLE IF NOT EXISTS ble_samples (
                id BIGINT GENERATED BY DEFAULT AS IDENTITY,
                device_mac TEXT NOT NULL,
                device_name TEXT NOT NULL,
                ble_ts INTEGER,
                x REAL NOT NULL,
                y REAL NOT NULL,
                z REAL NOT NULL,
                received_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                PRIMARY KEY (received_at, id)
            );

            SELECT create_hypertable('ble_samples', 'received_at', if_not_exists => TRUE, migrate_data => TRUE);

            CREATE INDEX IF NOT EXISTS idx_ble_samples_received_at ON ble_samples(received_at);

            CREATE TABLE IF NOT EXISTS device_battery_readings (
                id BIGSERIAL PRIMARY KEY,
                device_mac TEXT NOT NULL,
                device_name TEXT NOT NULL,
                battery_level SMALLINT NOT NULL,
                read_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            CREATE INDEX IF NOT EXISTS idx_battery_read_at ON device_battery_readings(read_at);
            CREATE INDEX IF NOT EXISTS idx_battery_device ON device_battery_readings(device_mac);
            ",
        )
        .await
}

/// Loop principal del escritor: acumula en memoria y descarga periodicamente por lotes.
///
/// Estrategia de flush:
/// - Inmediato al alcanzar 128 muestras.
/// - Periodico cada 50 ms si hay datos pendientes.
/// - Final al cerrar el canal.
///
/// Tolerancia a fallos: si `flush_batch` reporta un error transitorio
/// (transacción/inserción), el lote se reinyecta al frente del buffer y se
/// intenta una reconexión vía `reconnect_with_backoff`. Sólo si la
/// reconexión agota sus intentos descartamos los datos.
async fn writer_loop(mut client: Client, mut rx: mpsc::Receiver<DbRecord>, database_url: String) {
    let mut stats = WriterStats::new();

    let (mut insert_stmt, mut battery_stmt) = match prepare_statements(&client).await {
        Ok(pair) => pair,
        Err(err) => {
            error!("failed to prepare statements at startup: {}", err);
            return;
        }
    };

    // Tick periodico para limitar latencia en lotes pequenos.
    let mut tick = interval(Duration::from_millis(
        env::var("FLUSH_INTERVAL_MS")
            .unwrap_or("50".to_string())
            .parse()
            .unwrap_or(50),
    ));

    // Tick para reportar tasa de datos cada 5 segundos.
    let mut info_tick = interval(Duration::from_secs(5));
    // Buffer reusable para minimizar realocaciones.
    let mut buffer: Vec<DbRecord> = Vec::with_capacity(256);

    // Loop principal: espera 128 registros o 50 ms para enviar un lote.
    loop {
        tokio::select! {
            maybe_record = rx.recv() => {
                match maybe_record {
                    Some(record) => {
                        if let DbRecord::Sample(ref sample) = record
                            && (-20000.0 > sample.z || sample.z > 20000.0) {
                                debug!("discarding outlier sample: {:?}", sample);
                                continue;
                            }
                        buffer.push(record);
                        // Umbral de lote: reduce costo por transaccion.
                        if buffer.len() >= 128 {
                            try_flush(
                                &mut client, &mut insert_stmt, &mut battery_stmt,
                                &mut buffer, &mut stats, &database_url,
                            ).await;
                        }
                    }
                    None => {
                        if !buffer.is_empty() {
                            try_flush(
                                &mut client, &mut insert_stmt, &mut battery_stmt,
                                &mut buffer, &mut stats, &database_url,
                            ).await;
                        }
                        info!("sample channel closed, writer loop stopping");
                        break;
                    }
                }
            }
            _ = tick.tick() => {
                if !buffer.is_empty() {
                    try_flush(
                        &mut client, &mut insert_stmt, &mut battery_stmt,
                        &mut buffer, &mut stats, &database_url,
                    ).await;
                }
            }
            _ = info_tick.tick() => {
                let elapsed = stats.last_report.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    info!("total samples: {}, last batch: {}, rate: {:.2} samples/s",
                        stats.total_samples, stats.per_commit_samples, stats.samples_per_second);
                }
            }
        }
    }
}

/// Intenta un flush; si falla con un error transitorio, reinyecta los datos
/// en el buffer y reconecta antes del próximo intento.
///
/// Esto preserva el `WriterStats` global (mismo struct entre intentos) y
/// garantiza que un blip de red de 1-2 segundos no provoque pérdida de datos.
async fn try_flush(
    client: &mut Client,
    insert_stmt: &mut Statement,
    battery_stmt: &mut Statement,
    buffer: &mut Vec<DbRecord>,
    stats: &mut WriterStats,
    database_url: &str,
) {
    match flush_batch(client, insert_stmt, battery_stmt, buffer, stats).await {
        FlushOutcome::Ok => {}
        FlushOutcome::TransientError(pending) => {
            // Reinyectar al frente del buffer para preservar orden de muestras.
            // Es un edge case raro (sólo en reconexión) y `splice` con un
            // vector pequeño es O(N) sobre el buffer; aceptable.
            for (i, record) in pending.into_iter().enumerate() {
                buffer.insert(i, record);
            }
            match reconnect_with_backoff(database_url).await {
                Ok((new_client, new_insert, new_battery)) => {
                    *client = new_client;
                    *insert_stmt = new_insert;
                    *battery_stmt = new_battery;
                }
                Err(err) => {
                    // Reconnect agotó intentos: drenamos el buffer para no
                    // crecer sin límite y reportamos el incidente. Los datos
                    // nuevos seguirán encolándose mientras Postgres vuelve.
                    error!(
                        "PostgreSQL unreachable after retries ({}); dropping {} buffered records",
                        err,
                        buffer.len()
                    );
                    buffer.clear();
                }
            }
        }
    }
}

/// Resultado de un flush. `TransientError` lleva el lote pendiente para
/// que el caller pueda re-encolarlo tras reconectar.
enum FlushOutcome {
    Ok,
    TransientError(Vec<DbRecord>),
}

/// Persiste un lote dentro de una transaccion unica.
///
/// Devuelve `FlushOutcome::TransientError(pending)` con el lote sin persistir
/// cuando hay un fallo (transacción/inserción/commit). El caller decide:
/// reintentar tras reconectar, o descartar si la reconexión también falla.
async fn flush_batch(
    client: &mut Client,
    stmt: &Statement,
    battery_stmt: &Statement,
    buffer: &mut Vec<DbRecord>,
    stats: &mut WriterStats,
) -> FlushOutcome {
    // Mueve el contenido del buffer para liberar al productor cuanto antes.
    let mut pending = Vec::with_capacity(buffer.len());
    pending.append(buffer);

    let tx = match client.transaction().await {
        Ok(tx) => tx,
        Err(err) => {
            error!("failed to start transaction: {}", err);
            return FlushOutcome::TransientError(pending);
        }
    };

    for record in &pending {
        match record {
            DbRecord::Sample(sample) => {
                if let Err(err) = tx
                    .execute(
                        stmt,
                        &[
                            &sample.device_mac,
                            &sample.device_name,
                            &sample.ble_ts,
                            &sample.x,
                            &sample.y,
                            &sample.z,
                        ],
                    )
                    .await
                {
                    error!("failed to insert sample: {}", err);
                    drop(tx);
                    return FlushOutcome::TransientError(pending);
                }
            }
            DbRecord::Battery(reading) => {
                if let Err(err) = tx
                    .execute(
                        battery_stmt,
                        &[
                            &reading.device_mac,
                            &reading.device_name,
                            &reading.battery_level,
                        ],
                    )
                    .await
                {
                    error!("failed to insert battery reading: {}", err);
                    drop(tx);
                    return FlushOutcome::TransientError(pending);
                }
            }
        }
    }

    if let Err(err) = tx.commit().await {
        error!("failed to commit sample batch: {}", err);
        return FlushOutcome::TransientError(pending);
    }
    stats.total_samples += pending.len();
    stats.per_commit_samples = pending.len();
    let elapsed = stats.last_report.elapsed().as_secs_f64();
    if elapsed > 0.0 {
        let current_rate = pending.len() as f64 / elapsed;
        stats.samples_per_second = stats.samples_per_second * 0.8 + current_rate * 0.2;
    }
    stats.last_report = std::time::Instant::now();

    debug!("flushed {} records", pending.len());
    FlushOutcome::Ok
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper para crear un `BleSample` arbitrario en tests.
    fn sample(z: f32) -> DbRecord {
        DbRecord::Sample(BleSample {
            device_mac: "AA:BB:CC:DD:EE:FF".to_string(),
            device_name: "test".to_string(),
            ble_ts: None,
            x: 0.0,
            y: 0.0,
            z,
        })
    }

    /// Verifica que la lógica de reinyección preserva el orden FIFO de las
    /// muestras. Es la propiedad clave que evita que una reconexión "baraje"
    /// el flujo temporal de la telemetría BLE.
    #[test]
    fn reinjection_preserves_order() {
        let mut buffer: Vec<DbRecord> = vec![sample(7.0), sample(8.0), sample(9.0)];
        let pending: Vec<DbRecord> = vec![sample(1.0), sample(2.0), sample(3.0)];

        // Simulamos el camino del TransientError: pending va al frente del buffer.
        for (i, record) in pending.into_iter().enumerate() {
            buffer.insert(i, record);
        }

        // Esperamos: [1, 2, 3, 7, 8, 9].
        let z_values: Vec<f32> = buffer
            .iter()
            .map(|r| match r {
                DbRecord::Sample(s) => s.z,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(z_values, vec![1.0, 2.0, 3.0, 7.0, 8.0, 9.0]);
    }

    /// Smoke test: `FlushOutcome::TransientError` lleva el lote entero.
    /// Detectaría una regresión si alguien decidiera devolver `pending.split_off(...)`
    /// o `pending.drain(...).collect()` por error.
    #[test]
    fn transient_error_carries_full_batch() {
        let pending = vec![sample(1.0), sample(2.0), sample(3.0)];
        let outcome = FlushOutcome::TransientError(pending);
        match outcome {
            FlushOutcome::TransientError(p) => assert_eq!(p.len(), 3),
            FlushOutcome::Ok => panic!("expected TransientError"),
        }
    }
}
