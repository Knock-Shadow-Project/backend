use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use futures::stream::StreamExt;
use std::error::Error;
use tracing::{debug, info, warn};

pub(crate) async fn connect_device(mac: String) -> Result<Peripheral, Box<dyn Error>> {
    info!("Attempting to connect to device: {}", mac);

    let manager = Manager::new().await?;
    let central = manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or("No Bluetooth adapter found")?;

    info!("Starting Bluetooth scan");
    central.start_scan(ScanFilter::default()).await?;

    let peripherals = central.peripherals().await?;
    debug!("Found {} peripherals", peripherals.len());

    for peripheral in peripherals {
        let properties = peripheral.properties().await?;
        if let Some(props) = properties
            && props.address.to_string() == mac
        {
            info!("Device found: {}", mac);
            peripheral.connect().await?;
            info!("Successfully connected to: {}", mac);
            return Ok(peripheral);
        }
    }

    warn!("Device not found: {}", mac);
    Err("Device not found".into())
}

pub(crate) async fn stream_data(
    device: Peripheral,
) -> Result<impl futures::Stream<Item = Vec<u8>>, Box<dyn Error>> {
    info!("Discovering services");
    device.discover_services().await?;

    let characteristics = device.characteristics();
    info!("Found {} characteristics", characteristics.len());
    for characteristic in characteristics {
        debug!("Testing characteristic: {:?}", characteristic.uuid);
        let uuid_str = characteristic.uuid.to_string();
        if uuid_str.ends_with("-0001-11e1-ac36-0002a5d5c51b")
            && characteristic
                .properties
                .contains(btleplug::api::CharPropFlags::NOTIFY)
        {
            info!("Subscribing to characteristic: {:?}", characteristic.uuid);
            device.subscribe(&characteristic).await?;
        }
    }

    Ok(device
        .notifications()
        .await?
        .map(|notification| notification.value))
}

fn extract_timestamp(data: &[u8]) -> Option<(u16, &[u8])> {
    if data.len() < 2 {
        return None;
    }
    let timestamp = u16::from_le_bytes([data[0], data[1]]);
    Some((timestamp, &data[2..]))
}

pub fn decode_data(data: &[u8]) -> Option<(f32, f32, f32)> {
    let (_ts, payload) = match extract_timestamp(data) {
        Some(v) => (Some(v.0), v.1),
        None => (None, data),
    };
    if payload.len() < 6 {
        return None;
    }
    let x = i16::from_le_bytes([payload[0], payload[1]]) as f32;
    let y = i16::from_le_bytes([payload[2], payload[3]]) as f32;
    let z = i16::from_le_bytes([payload[4], payload[5]]) as f32;
    Some((x, y, z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_conn() {
        let mac = "DF:65:81:D0:D7:E5".to_string();
        info!("Starting connection test");
        let result = connect_device(mac).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream() {
        let mac = "DF:65:81:D0:D7:E5".to_string();
        info!("Starting stream test");
        let device = connect_device(mac).await.unwrap();
        let stream_result = stream_data(device).await;
        assert!(stream_result.is_ok());

        let mut stream = stream_result.unwrap();
        if let Some(data) = stream.next().await {
            info!("Received data: {} bytes", data.len());
            assert!(!data.is_empty());
        } else {
            panic!("No data received from the stream");
        }
    }
}
