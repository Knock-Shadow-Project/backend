//! Utilidades de Bluetooth Low Energy (BLE).
//!
//! Este modulo se encarga de:
//! - Buscar y conectar un dispositivo por direccion MAC.
//! - Suscribirse a notificaciones BLE de ciertas caracteristicas.
//! - Decodificar payloads binarios en valores numericos.

use btleplug::{
    api::{Central, Manager as _, Peripheral as _, ScanFilter},
    platform::{Manager, Peripheral},
};

use futures::stream::StreamExt;
use std::error::Error;
use tracing::{debug, info, warn};

/// UUID de la caracteristica Battery Level.
const BATTERY_LEVEL_CHAR_UUID: &str = "00002a19-0000-1000-8000-00805f9b34fb";

/// Busca un dispositivo BLE por MAC y devuelve su nombre y handle `Peripheral`.
///
/// Flujo:
/// 1. Obtiene el adaptador Bluetooth local.
/// 2. Inicia un escaneo de dispositivos.
/// 3. Recorre perifericos descubiertos y compara direccion MAC.
/// 4. Si coincide, conecta (si hace falta) y devuelve el dispositivo.
pub(crate) async fn connect_device(
    mac: String,
) -> Result<(String, Peripheral), Box<dyn Error + Send + Sync>> {
    info!("Connecting to device: {}", mac);

    // Crea el manager BLE y toma el primer adaptador disponible.
    let manager = Manager::new().await?;
    let central = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or("No Bluetooth adapter found")?;

    debug!("Discovering devices...");
    // Inicia escaneo con filtro por defecto (sin restricciones).
    central.start_scan(ScanFilter::default()).await?;

    let peripherals = central.peripherals().await?;
    debug!("Found {} peripherals", peripherals.len());

    // Revisa cada periferico para encontrar el MAC solicitado.
    for peripheral in peripherals {
        let properties = peripheral.properties().await?;
        if let Some(props) = properties
            && props.address.to_string() == mac
        {
            // Si no hay nombre publicitado, usa un valor por defecto.
            let name = props.local_name.clone().unwrap_or("Unknown".to_string());
            info!("Device found {} ({}), connecting...", name, mac);
            if peripheral.is_connected().await? {
                info!("Already connected to: {}", mac);
                return Ok((name, peripheral));
            }

            peripheral.connect().await?;
            info!("Successfully connected to: {}", mac);
            return Ok((name, peripheral));
        }
    }

    warn!("Device not found: {}", mac);
    Err("Device not found".into())
}

/// Descubre servicios, se suscribe a notificaciones y retorna un stream de bytes.
///
/// Solo se suscribe a caracteristicas que:
/// - Tienen UUID terminado en `-0001-11e1-ac36-0002a5d5c51b`.
/// - Exponen la propiedad `NOTIFY`.
pub(crate) async fn stream_data(
    device: &Peripheral,
) -> Result<impl futures::Stream<Item = Vec<u8>>, Box<dyn Error + Send + Sync>> {
    debug!("Discovering services");
    device.discover_services().await?;

    let characteristics = device.characteristics();
    debug!("Found {} characteristics", characteristics.len());
    // Filtra por el patron de UUID esperado y activa notificaciones.
    for characteristic in characteristics {
        debug!("Testing characteristic: {:?}", characteristic.uuid);
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

/// Intenta extraer un timestamp de 16 bits (little-endian) del inicio del buffer.
///
/// Devuelve una tupla con:
/// - `u16`: timestamp parseado.
/// - `&[u8]`: resto del payload sin el timestamp.
fn extract_timestamp(data: &[u8]) -> Option<(u16, &[u8])> {
    if data.len() < 2 {
        return None;
    }
    let timestamp = u16::from_le_bytes([data[0], data[1]]);
    Some((timestamp, &data[2..]))
}

/// Decodifica un frame binario en aceleracion `x`, `y`, `z` (i16 -> f32).
///
/// Formatos soportados:
/// - Con timestamp: `[ts_lo, ts_hi, x_lo, x_hi, y_lo, y_hi, z_lo, z_hi, ...]`
/// - Sin timestamp: `[x_lo, x_hi, y_lo, y_hi, z_lo, z_hi, ...]`
///
/// Retorna:
/// - `Some((Some(ts), x, y, z))` si habia timestamp y datos validos.
/// - `Some((None, x, y, z))` si no habia timestamp y datos validos.
/// - `None` si no hay suficientes bytes.
pub fn decode_data(data: &[u8]) -> Option<(Option<u16>, f32, f32, f32)> {
    let (ts, payload) = match extract_timestamp(data) {
        Some(v) => (Some(v.0), v.1),
        None => (None, data),
    };
    if payload.len() < 6 {
        return None;
    }
    let x = i16::from_le_bytes([payload[0], payload[1]]) as f32;
    let y = i16::from_le_bytes([payload[2], payload[3]]) as f32;
    let z = i16::from_le_bytes([payload[4], payload[5]]) as f32;
    Some((ts, x, y, z))
}

/// Lee el nivel de bateria desde el servicio estandar BLE Battery Service.
///
/// Devuelve `Some(u8)` con el porcentaje (0-100) si la caracteristica existe,
/// o `None` si no esta disponible.
pub async fn read_battery_level(device: &Peripheral) -> Result<Option<u8>, btleplug::Error> {
    debug!("Reading battery level...");
    // Asegura que los servicios esten descubiertos.
    if device.services().is_empty() {
        device.discover_services().await?;
    }

    for characteristic in device.characteristics() {
        let uuid_str = characteristic.uuid.to_string();
        if uuid_str == BATTERY_LEVEL_CHAR_UUID {
            debug!("Found battery level characteristic");
            match device.read(&characteristic).await {
                Ok(data) if !data.is_empty() => {
                    let level = data[0];
                    info!("Battery level read: {}%", level);
                    return Ok(Some(level));
                }
                Ok(_) => {
                    warn!("Battery characteristic returned empty data");
                    return Ok(None);
                }
                Err(err) => {
                    warn!("Failed to read battery level: {}", err);
                    return Err(err);
                }
            }
        }
    }

    debug!("Battery level characteristic not found");
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper que lee el MAC para tests de integración desde el entorno.
    ///
    /// Devuelve `None` (y el test debe ser skipped) si la variable no está
    /// definida — así `cargo test` funciona en CI sin hardware BLE, y solo se
    /// ejecuta el camino real cuando un desarrollador exporta su sensor.
    fn test_mac() -> Option<String> {
        std::env::var("TEST_DEVICE_MAC").ok()
    }

    /// Prueba unitaria pura: decoder de payloads BLE.
    #[test]
    fn decode_data_parses_payload_with_timestamp() {
        // ts=1, x=2, y=3, z=4 en little-endian.
        let raw = [0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0x00];
        let (ts, x, y, z) = decode_data(&raw).expect("valid payload should decode");
        assert_eq!(ts, Some(1));
        assert_eq!(x, 2.0);
        assert_eq!(y, 3.0);
        assert_eq!(z, 4.0);
    }

    /// Prueba unitaria pura: payload demasiado corto devuelve `None`.
    #[test]
    fn decode_data_rejects_short_payload() {
        assert!(decode_data(&[]).is_none());
        assert!(decode_data(&[0x01]).is_none());
    }

    /// Prueba de integracion: verifica que se puede conectar al dispositivo de prueba.
    /// Requiere `TEST_DEVICE_MAC=<MAC>` exportado y hardware disponible.
    #[tokio::test]
    #[ignore = "requires real BLE hardware; set TEST_DEVICE_MAC to run"]
    async fn test_conn() {
        let Some(mac) = test_mac() else {
            warn!("TEST_DEVICE_MAC not set; skipping integration test");
            return;
        };
        info!("Starting connection test for {}", mac);
        let result = connect_device(mac).await;
        assert!(
            result.is_ok(),
            "connection to test device should succeed: {:?}",
            result.err()
        );
    }

    /// Prueba de integracion: verifica que se pueda abrir el stream y recibir al menos un paquete.
    /// Requiere `TEST_DEVICE_MAC=<MAC>` exportado y hardware disponible.
    #[tokio::test]
    #[ignore = "requires real BLE hardware; set TEST_DEVICE_MAC to run"]
    async fn test_stream() {
        let Some(mac) = test_mac() else {
            warn!("TEST_DEVICE_MAC not set; skipping integration test");
            return;
        };
        info!("Starting stream test for {}", mac);
        let (_, device) = connect_device(mac)
            .await
            .expect("test device should connect");
        let mut stream = stream_data(&device)
            .await
            .expect("stream subscription should succeed");

        let data = stream
            .next()
            .await
            .expect("expected at least one BLE notification within timeout");
        info!("Received data: {} bytes", data.len());
        assert!(!data.is_empty(), "BLE payload should not be empty");
    }
}
