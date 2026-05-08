use axum::{
    Json, Router,
    extract::{Path, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use crate::{AppState, db};

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/", get(health))
        .route("/training/active", get(active_training))
        .route("/training/start", post(start_training))
        .route("/training/stop", post(stop_training))
        .route("/trainings/:id/punches", get(list_punches))
        .route("/live", get(ws_handler))
        .layer(cors)
        .with_state(state)
}

async fn health(State(_state): State<Arc<AppState>>) -> impl IntoResponse {
    axum::Json(serde_json::json!({ "status": "ok", "service": "pi-service" }))
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
                detected_at,
            },
        )
        .collect();

    Ok(axum::Json(punches))
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
