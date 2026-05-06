use futures::StreamExt;
use tokio::time::{interval, Duration};
use tracing::{debug, info, warn};

use crate::bt::decode_data;
use crate::storage::{BatteryReading, BleSample, DbWritter};

mod bt;
mod storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configura el subscriber global de tracing para ver logs en consola.
    tracing_subscriber::fmt()
        .with_max_level(if cfg!(debug_assertions) {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();

    // MAC del dispositivo objetivo.
    let mac = std::env::var("DEVICE_MAC").unwrap_or_else(|_| "DF:65:81:D0:D7:E5".to_string());
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow".to_string()
    });

    // Inicializa el escritor asincrono a PostgreSQL.
    let writer = DbWritter::connect(&database_url).await?;

    // 1) Conexion al dispositivo BLE.
    let (name, device) = bt::connect_device(mac.clone()).await?;
    info!("Connected to device: {}", name);

    // Clonar el peripheral para la tarea de bateria.
    let device_for_battery = device.clone();

    // 2) Apertura de stream de notificaciones.
    let mut stream = bt::stream_data(&device).await?;
    info!("Streaming BLE data to PostgreSQL...");

    // 3) Tarea de fondo: lee nivel de bateria cada 30 segundos.
    let writer_battery = writer.clone();
    let mac_battery = mac.clone();
    let name_battery = name.clone();
    tokio::spawn(async move {
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
                }
            }
        }
    });

    // 4) Procesamiento continuo de cada notificacion.
    while let Some(raw) = stream.next().await {
        if let Some((ble_ts, x, y, z)) = decode_data(&raw) {
            let sample = BleSample {
                device_mac: mac.clone(),
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
    Ok(())
}
