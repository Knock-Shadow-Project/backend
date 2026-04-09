use btleplug::api::Peripheral;
use futures::StreamExt;

use crate::bt::decode_data;

mod bt;
fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let mac = "DF:65:81:D0:D7:E5".to_string(); // Replace with a valid MAC address
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        match bt::connect_device(mac.clone()).await {
            Ok(device) => {
                println!("Connected to device: {:?}", device.characteristics());
                match bt::stream_data(device).await {
                    Ok(mut stream) => {
                        println!("Streaming data...");
                        while let Some(data) = stream.next().await {
                            println!("Received data: {:?}", decode_data(&data));
                        }
                    }
                    Err(e) => {
                        eprintln!("Error streaming data: {:?}", e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error connecting to device: {:?}", e);
            }
        }
    });
}
