use axum::{
    Json, Router,
    extract::{Path, State, WebSocketUpgrade},
    http::header,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::{AppState, db};

/// Embedded dashboard HTML served on `GET /`.
///
/// Baked into the binary at compile time via `include_str!` so the pi-service
/// ships a single artifact (no runtime filesystem dependency, no Dockerfile
/// changes). Re-builds are required to update the UI.
const INDEX_HTML: &str = include_str!("static/index.html");

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(index))
        .route("/health", get(health))
        .route("/sensors", get(sensors_status))
        .route("/training/active", get(active_training))
        .route("/training/start", post(start_training))
        .route("/training/stop", post(stop_training))
        .route("/trainings/{id}/punches", get(list_punches))
        .route("/live", get(ws_handler))
        .layer(cors)
        .with_state(state)
}

async fn index() -> impl IntoResponse {
    ([(header::CACHE_CONTROL, "no-store")], Html(INDEX_HTML))
}

async fn health(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    axum::Json(serde_json::json!({ "status": "ok", "service": "pi-service" }))
}

/// Estado por sensor para el UI: cuánto hace que enviaron datos.
///
/// Devuelve SIEMPRE los MACs configurados en `DEVICE_MAC_1/2`, aunque
/// alguno nunca haya escrito. `seconds_ago = null` ⇒ nunca visto.
/// El status lo computa el cliente desde `seconds_ago` (umbrales en el
/// HTML), así no tenemos que recompilar el servicio para ajustarlos.
#[derive(Serialize)]
struct SensorStatus {
    index: u8,
    mac: String,
    device_name: Option<String>,
    last_seen: Option<String>,
    seconds_ago: Option<f64>,
}

async fn sensors_status(
    State(state): State<Arc<AppState>>,
) -> Result<axum::Json<Vec<SensorStatus>>, (axum::http::StatusCode, String)> {
    let mut out = Vec::with_capacity(state.configured_macs.len());
    for s in &state.configured_macs {
        // received_at se guarda con strftime('%Y-%m-%d %H:%M:%f','now') (ver db.rs).
        // strftime('now','%s.%f') - max(juliantime) daría el delta pero sqlite
        // no expone fácil ese cálculo cross-timezone; hacemos el delta en Rust.
        let row: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT received_at, device_name
             FROM ble_samples
             WHERE device_mac = ?1
             ORDER BY received_at DESC
             LIMIT 1",
        )
        .bind(&s.mac)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        let (last_seen, device_name, seconds_ago) = match row {
            Some((ts, name)) => {
                // El timestamp viene en formato "YYYY-MM-DD HH:MM:SS.fff" en UTC.
                // Le adjuntamos "Z" para que el cliente pueda Date.parse() sin
                // ambigüedades de zona horaria.
                let parsed = chrono::NaiveDateTime::parse_from_str(&ts, "%Y-%m-%d %H:%M:%S%.f")
                    .ok()
                    .map(|n| n.and_utc());
                let secs = parsed.map(|t| {
                    (chrono::Utc::now() - t).num_milliseconds() as f64 / 1000.0
                });
                let iso = parsed.map(|t| t.to_rfc3339());
                (iso, name, secs)
            }
            None => (None, None, None),
        };

        out.push(SensorStatus {
            index: s.index,
            mac: s.mac.clone(),
            device_name,
            last_seen,
            seconds_ago,
        });
    }
    Ok(axum::Json(out))
}

#[derive(Deserialize)]
struct StartTrainingReq {
    user_id: i32,
    #[serde(default)]
    jwt: Option<String>,
    #[serde(default = "default_training_type")]
    training_type: String,
}

fn default_training_type() -> String {
    "Standard".to_string()
}

#[derive(Serialize)]
struct StartTrainingResp {
    local_training_id: i64,
}

async fn start_training(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StartTrainingReq>,
) -> Result<axum::Json<StartTrainingResp>, (axum::http::StatusCode, String)> {
    let id = db::create_training(&state.db, payload.user_id, &payload.training_type)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut active = state.active_training.lock().await;
    *active = Some(crate::ActiveTraining {
        local_id: id,
        user_id: payload.user_id,
        remote_jwt: payload.jwt,
        start_time: chrono::Utc::now(),
    });

    info!(
        "Training started: local_id={}, user_id={}",
        id, payload.user_id
    );
    Ok(axum::Json(StartTrainingResp {
        local_training_id: id,
    }))
}

#[derive(Deserialize)]
struct StopTrainingReq {
    local_training_id: i64,
}

#[derive(Serialize)]
struct StopTrainingResp {
    success: bool,
}

async fn stop_training(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StopTrainingReq>,
) -> Result<axum::Json<StopTrainingResp>, (axum::http::StatusCode, String)> {
    db::finish_training(&state.db, payload.local_training_id)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut active = state.active_training.lock().await;
    if let Some(ref a) = *active
        && a.local_id == payload.local_training_id
    {
        if let Some(ref jwt) = a.remote_jwt
            && let Some(ref _url) = state.remote_api_url
        {
            let payload_json = serde_json::json!({
                "end_time": chrono::Utc::now().to_rfc3339(),
            });
            let headers = serde_json::json!({
                "Authorization": format!("Bearer {}", jwt)
            })
            .to_string();
            let endpoint = format!("/trainings/{}", payload.local_training_id);
            let _ = db::enqueue_sync(
                &state.db,
                "PUT",
                &endpoint,
                &payload_json.to_string(),
                Some(&headers),
            )
            .await;
        }
        *active = None;
    }

    info!("Training stopped: local_id={}", payload.local_training_id);
    Ok(axum::Json(StopTrainingResp { success: true }))
}

#[derive(Serialize)]
struct ActiveTrainingResp {
    active: bool,
    local_training_id: Option<i64>,
    user_id: Option<i32>,
    start_time: Option<String>,
}

async fn active_training(
    State(state): State<Arc<AppState>>,
) -> Result<axum::Json<ActiveTrainingResp>, (axum::http::StatusCode, String)> {
    let active = state.active_training.lock().await;
    match *active {
        Some(ref a) => Ok(axum::Json(ActiveTrainingResp {
            active: true,
            local_training_id: Some(a.local_id),
            user_id: Some(a.user_id),
            start_time: Some(a.start_time.to_rfc3339()),
        })),
        None => Ok(axum::Json(ActiveTrainingResp {
            active: false,
            local_training_id: None,
            user_id: None,
            start_time: None,
        })),
    }
}

#[derive(Serialize)]
struct PunchRecord {
    id: i64,
    class_name: String,
    limb: Option<String>,
    position: Option<String>,
    power: Option<f64>,
    prob: Option<f64>,
    detected_at: String,
}

async fn list_punches(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<axum::Json<Vec<PunchRecord>>, (axum::http::StatusCode, String)> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            Option<String>,
            Option<String>,
            Option<f64>,
            Option<f64>,
            String,
        ),
    >(
        "SELECT id, class_name, limb, position, power, prob, detected_at
         FROM detected_punches WHERE local_training_id = ?1 ORDER BY detected_at ASC",
    )
    .bind(id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let punches: Vec<PunchRecord> = rows
        .into_iter()
        .map(
            |(id, class_name, limb, position, power, prob, detected_at)| PunchRecord {
                id,
                class_name,
                limb,
                position,
                power,
                prob,
                // detected_at en SQLite es `YYYY-MM-DD HH:MM:SS` (UTC, sin TZ).
                // El UI lo parsea con Date.parse(), que SIN sufijo Z asume hora
                // local del navegador → off-by-N-hours en zonas distintas a UTC.
                // Normalizamos a RFC3339 con `Z` para que el cliente no tenga
                // que adivinar la zona.
                detected_at: normalize_sqlite_ts_utc(&detected_at),
            },
        )
        .collect();

    Ok(axum::Json(punches))
}

/// Convierte un timestamp SQLite (`YYYY-MM-DD HH:MM:SS[.fff]`, sin TZ, asumido UTC)
/// a RFC3339 con sufijo `Z`. Si el string ya tiene zona horaria o no parsea,
/// se devuelve sin tocar — defensivo para no romper consumidores existentes.
fn normalize_sqlite_ts_utc(ts: &str) -> String {
    if ts.ends_with('Z') || ts.contains('+') || ts.contains('T') {
        return ts.to_string();
    }
    // Probar primero con fracción de segundo, luego sin.
    for fmt in ["%Y-%m-%d %H:%M:%S%.f", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(n) = chrono::NaiveDateTime::parse_from_str(ts, fmt) {
            return n.and_utc().to_rfc3339();
        }
    }
    ts.to_string()
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: axum::extract::ws::WebSocket, state: Arc<AppState>) {
    let mut rx = state.ws_tx.subscribe();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            Ok(event) = rx.recv() => {
                let text = serde_json::to_string(&event).unwrap_or_default();
                if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
                    break;
                }
            }
            else => break,
        }
    }
}
