use sqlx::{Pool, Sqlite};
use tracing::{info, warn};

pub async fn run(db: Pool<Sqlite>, api_base_url: String) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
    loop {
        interval.tick().await;
        if let Err(e) = sync_batch(&db, &api_base_url).await {
            warn!("Sync batch failed: {}", e);
        }
    }
}

async fn sync_batch(db: &Pool<Sqlite>, base_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let items = sqlx::query_as::<_, (i64, String, String, String, Option<String>)>(
        "SELECT id, method, endpoint, payload, headers FROM sync_queue ORDER BY id ASC LIMIT 50",
    )
    .fetch_all(db)
    .await?;

    if items.is_empty() {
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    for (id, method, endpoint, payload, headers) in items {
        let url = format!("{}{}", base_url.trim_end_matches('/'), endpoint);
        let mut req = match method.as_str() {
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "PATCH" => client.patch(&url),
            _ => client.post(&url),
        };

        req = req.body(payload).header("Content-Type", "application/json");

        if let Some(h) = headers
            && let Ok(map) = serde_json::from_str::<serde_json::Value>(&h)
            && let Some(obj) = map.as_object()
        {
            for (k, v) in obj {
                if let Some(vs) = v.as_str() {
                    req = req.header(k, vs);
                }
            }
        }

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                sqlx::query("DELETE FROM sync_queue WHERE id = ?1")
                    .bind(id)
                    .execute(db)
                    .await?;
                info!("Synced queue item {}", id);
            }
            Ok(resp) => {
                warn!("Sync item {} got HTTP {}", id, resp.status());
                // No borramos para reintentar luego; se podría añadir backoff o dead-letter
            }
            Err(e) => {
                warn!("Sync item {} network error: {}", id, e);
                return Err(Box::new(e));
            }
        }
    }

    Ok(())
}
