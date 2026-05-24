//! Utilidades de Bluetooth Low Energy (BLE) para pi-service.
//!
//! Soporta conexion a 1 o 2 sensores y guarda muestras en SQLite local.

use btleplug::{
    api::{Central, Manager as _, Peripheral as _, ScanFilter},
    platform::{Manager, Peripheral},
};
use futures::stream::StreamExt;
use std::error::Error;
use tracing::{debug, error, info, warn};

const BATTERY_LEVEL_CHAR_UUID: &str = "00002a19-0000-1000-8000-00805f9b34fb";

/// Conecta a los dispositivos BLE configurados y arranca el loop de recepcion.
pub async fn run_ble(
    mac1: Option<String>,
    mac2: Option<String>,
    state: std::sync::Arc<crate::AppState>,
) -> Result<(), Box<dyn Error>> {
    let macs: Vec<String> = vec![mac1, mac2].into_iter().flatten().collect();
    if macs.is_empty() {
        warn!("No DEVICE_MAC configured, BLE task will not start");
        return Ok(());
    }

    let manager = Manager::new().await?;
    let central = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or("No Bluetooth adapter found")?;

    info!("Starting BLE scan for {} device(s)", macs.len());
    central.start_scan(ScanFilter::default()).await?;

    // Pequena pausa para que el scan descubra dispositivos
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    let peripherals = central.peripherals().await?;
    let mut devices: Vec<(String, String, Peripheral)> = Vec::new(); // (mac, name, peripheral)

    for peripheral in peripherals {
        let properties = peripheral.properties().await?;
        if let Some(props) = properties {
            let addr = props.address.to_string();
            if macs.contains(&addr) && !devices.iter().any(|(m, _, _)| m == &addr) {
                let name = props
                    .local_name
                    .clone()
                    .unwrap_or_else(|| "Unknown".to_string());
                if !peripheral.is_connected().await? {
                    info!("Connecting to {} ({})...", name, addr);
                    peripheral.connect().await?;
                }
                info!("Connected to {} ({})", name, addr);
                devices.push((addr, name, peripheral));
                if devices.len() == macs.len() {
                    break;
                }
            }
        }
    }

    if devices.len() != macs.len() {
        return Err(format!(
            "Only found {} of {} requested devices",
            devices.len(),
            macs.len()
        )
        .into());
    }

    // Suscribir notificaciones para todos los dispositivos
    let mut streams = Vec::new();
    for (mac, name, device) in &devices {
        let stream = stream_data(device).await?;
        streams.push((mac.clone(), name.clone(), stream));
    }

    // Tarea de bateria para cada dispositivo
    for (mac, name, device) in &devices {
        let mac = mac.clone();
        let name = name.clone();
        let device = device.clone();
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                tick.tick().await;
                match read_battery_level(&device).await {
                    Ok(Some(level)) => {
                        debug!("Battery {} ({}): {}%", name, mac, level);
                    }
                    Ok(None) => {
                        debug!("Battery level not available for {}", name);
                    }
                    Err(e) => {
                        warn!("Failed to read battery for {}: {}", name, e);
                    }
                }
            }
        });
    }

    // Merge de streams: emite tuplas (mac, name, raw_bytes)
    let mut merged = futures::stream::select_all(
        streams
            .into_iter()
            .map(|(mac, name, stream)| stream.map(move |raw| (mac.clone(), name.clone(), raw))),
    );

    while let Some((mac, name, raw)) = merged.next().await {
        if let Some((ble_ts, x, y, z)) = decode_data(&raw) {
            if let Err(e) =
                crate::db::insert_ble_sample(&state.db, &mac, &name, ble_ts.map(i32::from), x, y, z)
                    .await
            {
                error!("Failed to insert BLE sample: {}", e);
            }
            let _ = state.accel_tx.send(crate::AccelSample {
                mac: mac.clone(),
                device_name: name.clone(),
                x,
                y,
                z,
                ts: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64,
            });
        }
    }

    warn!("BLE stream ended");
    Ok(())
}

async fn stream_data(
    device: &Peripheral,
) -> Result<impl futures::Stream<Item = Vec<u8>>, Box<dyn Error>> {
    device.discover_services().await?;
    let characteristics = device.characteristics();
    for characteristic in characteristics {
        let uuid_str = characteristic.uuid.to_string();
        if uuid_str.ends_with("-0001-11e1-ac36-0002a5d5c51b")
            && characteristic
                .properties
                .contains(btleplug::api::CharPropFlags::NOTIFY)
        {
            debug!("Subscribing to characteristic: {:?}", characteristic.uuid);
            device.subscribe(&characteristic).await?;
        }
    }
    Ok(device
        .notifications()
        .await?
        .map(|notification| notification.value))
}

fn decode_data(data: &[u8]) -> Option<(Option<u16>, f32, f32, f32)> {
    if data.len() < 6 {
        return None;
    }
    let has_ts = data.len() >= 8;
    let (ts, payload) = if has_ts {
        let t = u16::from_le_bytes([data[0], data[1]]);
        (Some(t), &data[2..8])
    } else {
        (None, &data[0..6])
    };
    let x = i16::from_le_bytes([payload[0], payload[1]]) as f32;
    let y = i16::from_le_bytes([payload[2], payload[3]]) as f32;
    let z = i16::from_le_bytes([payload[4], payload[5]]) as f32;

    // Two failure modes observed on these BLE sensors:
    //   1. i16 saturation: axis pinned to ±32768 (firmware clip on bad I²C).
    //   2. Framing glitches: a plausible-looking value in one axis combined
    //      with garbage in the others — e.g. (640, 3905, -32768). Each axis
    //      passes a simple per-axis range check, but the magnitude is
    //      physically impossible (~33 G total) for a human punch (peaks ~10 G).
    // Filter both with a per-axis bound (sensor is ±16 G ≈ ±16000 raw, plus
    // margin) AND a total-magnitude bound. Keep limits loose enough that real
    // hard hits pass.
    const AXIS_LIMIT: f32 = 16_500.0;
    if !(-AXIS_LIMIT..=AXIS_LIMIT).contains(&x)
        || !(-AXIS_LIMIT..=AXIS_LIMIT).contains(&y)
        || !(-AXIS_LIMIT..=AXIS_LIMIT).contains(&z)
    {
        return None;
    }
    const MAX_MAG_SQ: f32 = 18_000.0 * 18_000.0;
    if x * x + y * y + z * z > MAX_MAG_SQ {
        return None;
    }

    Some((ts, x, y, z))
}

pub async fn read_battery_level(device: &Peripheral) -> Result<Option<u8>, btleplug::Error> {
    if device.services().is_empty() {
        device.discover_services().await?;
    }
    for characteristic in device.characteristics() {
        if characteristic.uuid.to_string() == BATTERY_LEVEL_CHAR_UUID {
            match device.read(&characteristic).await {
                Ok(data) if !data.is_empty() => return Ok(Some(data[0])),
                Ok(_) => return Ok(None),
                Err(e) => return Err(e),
            }
        }
    }
    Ok(None)
}
