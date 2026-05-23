use std::env;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use tracing::info;

mod api;
mod bt;
mod db;
mod mdns;
mod sync;

#[derive(Clone)]
pub struct AppState {
    pub db: sqlx::SqlitePool,
    pub ws_tx: Arc<broadcast::Sender<PunchEvent>>,
    pub active_training: Arc<Mutex<Option<ActiveTraining>>>,
    pub remote_api_url: Option<String>,
    /// MACs configurados vía DEVICE_MAC_1/2. Se exponen en `/sensors` para
    /// que el UI pueda mostrar online/offline aunque un sensor lleve horas
    /// caído (un MAC sin ninguna fila histórica seguirá listado).
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

#[derive(Clone, Debug, serde::Serialize)]
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

    let state = Arc::new(AppState {
        db: pool.clone(),
        ws_tx: Arc::new(ws_tx),
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

    // Punch broadcaster task: lee detected_punches y emite por WS
    let state_clone = state.clone();
    tokio::spawn(async move {
        let mut last_id: i64 = 0;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        loop {
            interval.tick().await;
            let rows =
                sqlx::query_as::<_, (i64, String, String, String, Option<f64>, f64, String)>(
                    "SELECT id, class_name, limb, position, power, prob, detected_at
                 FROM detected_punches WHERE id > ?1 ORDER BY id ASC",
                )
                .bind(last_id)
                .fetch_all(&state_clone.db)
                .await;

            match rows {
                Ok(punches) => {
                    for (id, class_name, limb, position, power, prob, detected_at) in punches {
                        last_id = id;
                        let event = PunchEvent {
                            class_name,
                            limb,
                            position,
                            power,
                            prob,
                            detected_at,
                        };
                        let _ = state_clone.ws_tx.send(event);
                    }
                }
                Err(e) => {
                    tracing::debug!("Punch poll error: {}", e);
                }
            }
        }
    });

    // HTTP API
    let app = api::router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("PI service running on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await?;

    Ok(())
}
