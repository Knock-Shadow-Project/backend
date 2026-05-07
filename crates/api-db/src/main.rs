use std::env;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::info;

mod auth;
mod db;
mod models;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://knockshadow:knockshadow@127.0.0.1:5432/knockshadow".to_string()
    });
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());

    let pool = db::connect(&database_url).await?;
    info!("Connected to database");

    let (ws_tx, _ws_rx) = broadcast::channel::<String>(256);
    let app_state = AppState {
        pool,
        ws_tx: Arc::new(ws_tx),
    };

    let app = routes::router().with_state(app_state);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("Server running on http://0.0.0.0:{}", port);
    axum::serve(listener, app).await?;

    Ok(())
}
