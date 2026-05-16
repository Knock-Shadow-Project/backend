use std::sync::Arc;

use axum::{Router, routing::get};
use axum_prometheus::PrometheusMetricLayer;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::info;

mod auth;
mod config;
mod db;
mod models;
mod routes;
mod state;

use config::Config;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Fail-fast on missing required secrets so we never serve traffic with an
    // insecure default. The JWT secret is loaded into a process-wide cell and
    // re-used by auth handlers/middleware.
    if let Err(err) = auth::init_jwt_secret() {
        tracing::error!("{}", err);
        return Err(err.into());
    }

    // Carga tipada del resto del entorno. Centralizada en `config::Config`
    // para que añadir un nuevo setting sea un único cambio testeable.
    let cfg = Config::from_env()?;

    let pool = db::connect(&cfg.database_url).await?;
    info!("Connected to database");

    let (ws_tx, _ws_rx) = broadcast::channel::<String>(256);
    let app_state = AppState {
        pool,
        ws_tx: Arc::new(ws_tx),
    };

    // Métricas Prometheus: el layer recoge automáticamente histogramas de
    // latencia y contadores de status para cada ruta HTTP. El handle expone
    // el endpoint /metrics en formato text/plain estándar.
    //
    // Convención: /metrics queda fuera del auth middleware para que el
    // scraper de Prometheus pueda llegar sin JWT. Si más adelante se
    // expone públicamente, restringirlo por IP o auth básico en el reverse
    // proxy en vez de aquí.
    let (prometheus_layer, metric_handle) = PrometheusMetricLayer::pair();

    let app = Router::new()
        .route(
            "/metrics",
            get(move || async move { metric_handle.render() }),
        )
        .merge(routes::router().with_state(app_state))
        .layer(prometheus_layer);

    let listener = TcpListener::bind(format!("0.0.0.0:{}", cfg.port)).await?;
    info!("Server running on http://0.0.0.0:{}", cfg.port);
    info!("Prometheus metrics exposed at /metrics");
    axum::serve(listener, app).await?;

    Ok(())
}
