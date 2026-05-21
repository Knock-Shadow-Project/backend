use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::str::FromStr;
use std::time::Duration;
use tracing::info;

pub async fn init(db_path: &str) -> Result<Pool<Sqlite>, sqlx::Error> {
    // WAL + busy_timeout are required: this file is shared via a Docker volume
    // with the Python pi-inference and ml-app containers. Without WAL the
    // default rollback-journal serializes everything; without busy_timeout the
    // first conflicting writer sees SQLITE_BUSY immediately.
    let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(5))
        .foreign_keys(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS ble_samples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            device_mac TEXT NOT NULL,
            device_name TEXT NOT NULL,
            ble_ts INTEGER,
            x REAL NOT NULL,
            y REAL NOT NULL,
            z REAL NOT NULL,
            received_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );

        CREATE TABLE IF NOT EXISTS detected_punches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER,
            local_training_id INTEGER,
            class_name TEXT NOT NULL,
            limb TEXT,
            position TEXT,
            power REAL,
            prob REAL,
            detected_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            synced BOOLEAN NOT NULL DEFAULT FALSE
        );

        CREATE TABLE IF NOT EXISTS local_trainings (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            start_time DATETIME NOT NULL,
            end_time DATETIME,
            training_type TEXT,
            synced BOOLEAN NOT NULL DEFAULT FALSE,
            remote_training_id INTEGER
        );

        CREATE TABLE IF NOT EXISTS sync_queue (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            method TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            payload TEXT NOT NULL,
            headers TEXT,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
            attempts INTEGER NOT NULL DEFAULT 0,
            last_attempt_at DATETIME
        );

        CREATE INDEX IF NOT EXISTS idx_ble_samples_time ON ble_samples(received_at);
        CREATE INDEX IF NOT EXISTS idx_punches_training ON detected_punches(local_training_id);
        CREATE INDEX IF NOT EXISTS idx_sync_queue_created ON sync_queue(created_at);
        CREATE INDEX IF NOT EXISTS idx_sync_queue_retry ON sync_queue(last_attempt_at, attempts);",
    )
    .execute(&pool)
    .await?;

    // Migración idempotente: añadir columnas a sync_queue si la tabla existía
    // antes de este cambio. `ALTER TABLE ADD COLUMN` falla si la columna ya
    // existe, así que ignoramos ese error específico.
    for stmt in [
        "ALTER TABLE sync_queue ADD COLUMN attempts INTEGER NOT NULL DEFAULT 0",
        "ALTER TABLE sync_queue ADD COLUMN last_attempt_at DATETIME",
    ] {
        if let Err(e) = sqlx::query(stmt).execute(&pool).await {
            // SQLite devuelve "duplicate column name" si ya existe — esperado.
            let msg = e.to_string();
            if !msg.contains("duplicate column") {
                return Err(e);
            }
        }
    }

    info!("SQLite schema ready");
    Ok(pool)
}

pub async fn insert_ble_sample(
    pool: &Pool<Sqlite>,
    mac: &str,
    name: &str,
    ble_ts: Option<i32>,
    x: f32,
    y: f32,
    z: f32,
) -> Result<(), sqlx::Error> {
    // `received_at` se setea explícito con resolución de milisegundos.
    // El DEFAULT del schema es `CURRENT_TIMESTAMP`, que en SQLite tiene
    // resolución de 1 segundo: con ~50 muestras/s por sensor, todas
    // colisionaban en el mismo timestamp y rompían el `merge_asof` de
    // `ml/pipeline/_signal.py` (todas las muestras de un sensor en ese
    // segundo se emparejaban con UNA sola muestra del otro). Ver
    // `ml/pipeline/_signal.py::_spread_duplicate_timestamps` para el
    // workaround que reparte datos antiguos sin sub-segundo.
    sqlx::query(
        "INSERT INTO ble_samples (device_mac, device_name, ble_ts, x, y, z, received_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%d %H:%M:%f','now'))",
    )
    .bind(mac)
    .bind(name)
    .bind(ble_ts)
    .bind(x)
    .bind(y)
    .bind(z)
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub async fn insert_punch(
    pool: &Pool<Sqlite>,
    user_id: Option<i32>,
    local_training_id: Option<i64>,
    class_name: &str,
    limb: &str,
    position: &str,
    power: Option<f64>,
    prob: f64,
) -> Result<i64, sqlx::Error> {
    let id = sqlx::query(
        "INSERT INTO detected_punches
         (user_id, local_training_id, class_name, limb, position, power, prob)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(user_id)
    .bind(local_training_id)
    .bind(class_name)
    .bind(limb)
    .bind(position)
    .bind(power)
    .bind(prob)
    .execute(pool)
    .await?
    .last_insert_rowid();
    Ok(id)
}

pub async fn create_training(
    pool: &Pool<Sqlite>,
    user_id: i32,
    training_type: &str,
) -> Result<i64, sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    let id = sqlx::query(
        "INSERT INTO local_trainings (user_id, start_time, training_type)
         VALUES (?1, ?2, ?3)",
    )
    .bind(user_id)
    .bind(now)
    .bind(training_type)
    .execute(pool)
    .await?
    .last_insert_rowid();
    Ok(id)
}

pub async fn finish_training(pool: &Pool<Sqlite>, id: i64) -> Result<(), sqlx::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query("UPDATE local_trainings SET end_time = ?1 WHERE id = ?2")
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn enqueue_sync(
    pool: &Pool<Sqlite>,
    method: &str,
    endpoint: &str,
    payload: &str,
    headers: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO sync_queue (method, endpoint, payload, headers)
         VALUES (?1, ?2, ?3, ?4)",
    )
    .bind(method)
    .bind(endpoint)
    .bind(payload)
    .bind(headers)
    .execute(pool)
    .await?;
    Ok(())
}
