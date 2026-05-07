use std::sync::Arc;
use tokio::sync::broadcast;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub ws_tx: Arc<broadcast::Sender<String>>,
}
