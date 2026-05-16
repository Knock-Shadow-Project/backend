use futures::StreamExt;
use tokio::time::{interval, sleep, Duration};
use tracing::{debug, error, info, warn};

use crate::bt::decode_data;
use crate::storage::{BatteryReading, BleSample, DbWritter};

mod bt;
mod storage;

/// Initial backoff between BLE reconnection attempts.
const BLE_BACKOFF_MIN: Duration = Duration::from_secs(2);
/// Maximum backoff between BLE reconnection attempts.
const BLE_BACKOFF_MAX: Duration = Duration::from_secs(60);

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Configura el subscriber global de tracing para ver logs en consola.
    tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();

    // MAC del dispositivo objetivo — requerido por env para evitar conexiones
    // accidentales a sensores ajenos o defaults filtrados al repo.
    let mac = std::env::var("DEVICE_MAC").map_err(|_| {
        "DEVICE_MAC environment variable is required (e.g. DEVICE_MAC=DF:65:81:D0:D7:E5)"
    })?;
    // DATABASE_URL fail-fast: arrastrar credenciales hardcodeadas como fallback
    // es una mala práctica y deja la base de datos abierta a errores de despliegue.
    let database_url = std::env::var("DATABASE_URL").map_err(|_| {
        "DATABASE_URL environment variable is required (e.g. \
         postgres://user:pass@host:5432/db)"
    })?;

    // Inicializa el escritor asincrono a PostgreSQL.
    let writer = DbWritter::connect(&database_url).await?;

    // Bucle de reconexión: cualquier error o cierre del stream BLE provoca un
    // reintento con backoff exponencial en vez de matar el proceso. Esto cubre
    // pérdidas de señal, reinicios del sensor o pausas BLE del adaptador.
    let mut backoff = BLE_BACKOFF_MIN;
    loop {
        match run_session(&mac, &writer).await {
            Ok(()) => {
                warn!("BLE stream ended cleanly; reconnecting in {:?}", backoff);
            }
            Err(err) => {
                error!("BLE session failed: {}; reconnecting in {:?}", err, backoff);
            }
        }
        sleep(backoff).await;
        // Backoff exponencial con cap; al primer éxito sostenido se reinicia
        // dentro de run_session (ver `backoff = BLE_BACKOFF_MIN` debajo).
        backoff = (backoff * 2).min(BLE_BACKOFF_MAX);
        if writer.is_closed() {
            warn!("DB writer channel closed; exiting BLE loop");
            return Ok(());
        }
    }
}

/// Ejecuta una sesión completa de conexión + streaming + lectura de batería.
///
/// Vuelve a `Ok(())` cuando el stream BLE finaliza limpiamente; vuelve a
/// `Err(...)` si falla la conexión, el descubrimiento o la suscripción.
async fn run_session(
    mac: &str,
    writer: &DbWritter,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 1) Conexion al dispositivo BLE.
    let (name, device) = bt::connect_device(mac.to_string()).await?;
    info!("Connected to device: {}", name);

    // Clonar el peripheral para la tarea de bateria.
    let device_for_battery = device.clone();

    // 2) Apertura de stream de notificaciones.
    let mut stream = bt::stream_data(&device).await?;
    info!("Streaming BLE data to PostgreSQL...");

    // 3) Tarea de fondo: lee nivel de bateria cada 30 segundos.
    //    Termina automáticamente al cerrar el canal del writer o al finalizar
    //    la sesión (drop del Peripheral cuando salimos del scope).
    let writer_battery = writer.clone();
    let mac_battery = mac.to_string();
    let name_battery = name.clone();
    let battery_task = tokio::spawn(async move {
        let mut tick = interval(Duration::from_secs(30));
        loop {
            tick.tick().await;
            match bt::read_battery_level(&device_for_battery).await {
                Ok(Some(level)) => {
                    let reading = BatteryReading {
                        device_mac: mac_battery.clone(),
                        device_name: name_battery.clone(),
                        battery_level: level as i16,
                    };
                    if let Err(err) = writer_battery.send_battery(reading).await {
                        debug!("battery writer queue closed: {}", err);
                        break;
                    }
                }
                Ok(None) => {
                    debug!("battery level characteristic not available");
                }
                Err(err) => {
                    warn!("failed to read battery level: {}", err);
                    // Si la lectura falla por desconexión, salimos del loop
                    // para que la sesión completa pueda reiniciarse.
                    break;
                }
            }
        }
    });

    // 4) Procesamiento continuo de cada notificacion.
    while let Some(raw) = stream.next().await {
        if let Some((ble_ts, x, y, z)) = decode_data(&raw) {
            let sample = BleSample {
                device_mac: mac.to_string(),
                device_name: name.clone(),
                ble_ts: ble_ts.map(i32::from),
                x,
                y,
                z,
            };
            if let Err(err) = writer.send_sample(sample).await {
                debug!("writer queue closed: {}", err);
                break;
            }
        }
    }

    // Forzamos la cancelación de la tarea de batería para no dejarla huérfana
    // entre reconexiones (el Peripheral se descarta al volver de la función).
    battery_task.abort();

    Ok(())
}
