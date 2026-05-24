use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::email_service::EmailService;
use crate::rate_limit::RateLimiter;

/// Estado compartido por todos los handlers de Axum.
///
/// `email_service` es `Option` porque sin `RESEND_API_KEY` el servicio no
/// se inicializa y los handlers caen al modo stub.
///
/// `rate_limiter` siempre está presente (al menos en modo in-memory) para
/// que los handlers no tengan que lidiar con `Option`. Con `REDIS_URL` se
/// configura el backend distribuido al arrancar.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub ws_tx: Arc<broadcast::Sender<String>>,
    pub email_service: Option<EmailService>,
    pub rate_limiter: RateLimiter,
}
