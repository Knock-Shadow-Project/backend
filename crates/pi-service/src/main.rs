use std::env;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tracing::info;

mod api;
mod bt;
mod db;
mod mdns;
mod sync;

#[derive(Clone, Debug, serde::Serialize)]
pub struct AccelSample {
    pub mac: String,
    pub device_name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub ts: u64,
}

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub ws_tx: Arc<broadcast::Sender<PunchEvent>>,
    pub accel_tx: Arc<broadcast::Sender<AccelSample>>,
    pub active_training: Arc<Mutex<Option<ActiveTraining>>>,
    pub remote_api_url: Option<String>,
    pub configured_macs: Vec<ConfiguredSensor>,
}

#[derive(Clone, Debug)]
pub struct ConfiguredSensor {
    pub index: u8,
    pub mac: String,
}

pub struct ActiveTraining {
    pub local_id: i64,
    pub user_id: i32,
    pub remote_jwt: Option<String>,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PunchEvent {
    pub class_name: String,
    pub limb: String,
    pub position: String,
    pub power: Option<f64>,
    pub prob: f64,
    pub detected_at: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().init();

    let db_path = env::var("DB_PATH").unwrap_or_else(|_| "pi_data.db".to_string());
    let pool = db::init(&db_path).await?;
    info!("SQLite initialized at {}", db_path);

    let (ws_tx, _ws_rx) = broadcast::channel::<PunchEvent>(256);
    let (accel_tx, _accel_rx) = broadcast::channel::<AccelSample>(512);

    // Recoge los MACs configurados antes de mover los Option<String> a la
    // task BLE: el endpoint /sensors necesita listar SIEMPRE los configurados,
    // incluso si nunca han enviado datos, para distinguir "sensor caído" de
    // "sensor no configurado".
    let mac1_env = env::var("DEVICE_MAC_1").ok();
    let mac2_env = env::var("DEVICE_MAC_2").ok();
    let configured_macs: Vec<ConfiguredSensor> = [(1, &mac1_env), (2, &mac2_env)]
        .into_iter()
        .filter_map(|(idx, m)| {
            m.as_ref().map(|mac| ConfiguredSensor {
                index: idx,
                mac: mac.clone(),
            })
        })
        .collect();

    let accel_tx_arc = Arc::new(accel_tx);

    let state = Arc::new(AppState {
        db: pool.clone(),
        ws_tx: Arc::new(ws_tx),
        accel_tx: accel_tx_arc.clone(),
        active_training: Arc::new(Mutex::new(None)),
        remote_api_url: env::var("API_BASE_URL").ok(),
        configured_macs,
    });

    // mDNS discovery — delegated to the host's avahi-daemon via DBus
    // (see crates/pi-service/src/mdns.rs). The returned handle must stay
    // bound for the lifetime of the process; dropping it kills the child
    // and de-registers the service.
    let mdns_hostname = env::var("MDNS_HOSTNAME").unwrap_or_else(|_| "knockshadow-pi".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()?;
    let _mdns = mdns::announce(&mdns_hostname, port)?;
    info!("mDNS announced {} on port {}", mdns_hostname, port);

    // Remote sync loop
    if let Some(ref url) = state.remote_api_url {
        let db_clone = pool.clone();
        let url_clone = url.clone();
        tokio::spawn(async move {
            sync::run(db_clone, url_clone).await;
        });
        info!("Remote sync enabled: {}", url);
    }

    // BLE ingestion task
    let mac1 = mac1_env;
    let mac2 = mac2_env;
    if mac1.is_some() || mac2.is_some() {
        let state_clone = state.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = bt::run_ble(mac1.clone(), mac2.clone(), state_clone.clone()).await {
                    tracing::error!("BLE error: {}; retrying in 5s...", e);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        });
        info!("BLE task started");
    } else {
        info!("No DEVICE_MAC configured, BLE task skipped");
    }

    // MQTT: publish accel data + subscribe to punch events from pi-inference.
    // Punch events arrive via topic `knockshadow/punches` (published by
    // pi-inference) and are forwarded to the WS broadcast channel, replacing
    // the old SQLite polling loop. Accel data is still published for external
    // consumers (dashboards, future features).
    {
        let mqtt_host = env::var("MQTT_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let mqtt_port: u16 = env::var("MQTT_PORT")
            .unwrap_or_else(|_| "1883".to_string())
            .parse()
            .unwrap_or(1883);
        info!("MQTT started (broker {mqtt_host}:{mqtt_port})");

        let mut mqtt_rx = accel_tx_arc.subscribe();
        let ws_tx_mqtt = state.ws_tx.clone();

        tokio::spawn(async move {
            let mut mqttoptions = rumqttc::MqttOptions::new("pi-service", &mqtt_host, mqtt_port);
            mqttoptions.set_keep_alive(std::time::Duration::from_secs(30));
            let (client, mut eventloop) = rumqttc::AsyncClient::new(mqttoptions, 256);

            if let Err(e) = client
                .subscribe("knockshadow/punches", rumqttc::QoS::AtLeastOnce)
                .await
            {
                tracing::error!("MQTT subscribe to knockshadow/punches failed: {e}");
            }

            // Accel publisher task (downsample 1:5)
            let client_pub = client.clone();
            tokio::spawn(async move {
                let mut skip_counter: u32 = 0;
                loop {
                    match mqtt_rx.recv().await {
                        Ok(sample) => {
                            skip_counter += 1;
                            if !skip_counter.is_multiple_of(5) {
                                continue;
                            }
                            let topic = format!(
                                "knockshadow/sensors/{}/accel",
                                sample.mac.replace(':', "")
                            );
                            let payload = serde_json::to_string(&sample).unwrap_or_default();
                            let _ = client_pub
                                .publish(&topic, rumqttc::QoS::AtMostOnce, false, payload)
                                .await;
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            tracing::debug!("MQTT lagged {n} accel samples");
                        }
                        Err(_) => break,
                    }
                }
            });

            // Eventloop driver: handles reconnection + incoming punch messages
            loop {
                match eventloop.poll().await {
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::ConnAck(ack))) => {
                        info!(
                            "MQTT connected to broker {mqtt_host}:{mqtt_port} (code={:?})",
                            ack.code
                        );
                    }
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::SubAck(_))) => {
                        info!("MQTT subscribed to knockshadow/punches");
                    }
                    Ok(rumqttc::Event::Incoming(rumqttc::Packet::Publish(msg))) => {
                        if msg.topic == "knockshadow/punches" {
                            match serde_json::from_slice::<PunchEvent>(&msg.payload) {
                                Ok(event) => {
                                    let n = ws_tx_mqtt.receiver_count();
                                    info!(
                                        "MQTT punch received ({} -> {} WS clients)",
                                        event.class_name, n
                                    );
                                    let _ = ws_tx_mqtt.send(event);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Invalid punch MQTT payload: {e}; raw={}",
                                        String::from_utf8_lossy(&msg.payload)
                                    );
                                }
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("MQTT eventloop error: {e}; reconnecting in 3s");
                        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                    }
                }
            }
        });
    }

    // HTTP API
    let app = api::router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("PI service running on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await?;

    Ok(())
}
