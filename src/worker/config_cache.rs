use anyhow::Result;
use once_cell::sync::Lazy;
use tokio::sync::Mutex;
use lru_time_cache::LruCache;
use uuid::Uuid;
use serde_json::Value as JsonValue;
use log::trace;

// TODO - broadcast invalidation from the API

static PROJ_CONFIG_CACHE: Lazy<Mutex<LruCache<Uuid, JsonValue>>> =
    Lazy::new(|| Mutex::new(LruCache::with_expiry_duration_and_capacity(
        chrono::Duration::hours(24).to_std().unwrap(),
        100,
    )));

pub async fn get_project_config(proj_id: Uuid) -> Result<JsonValue> {
    let mut cache = PROJ_CONFIG_CACHE.lock().await;
    let config = cache.get(&proj_id);

    if let Some(config) = config {
        return Ok(config.clone());
    }

    // cache miss
    let config = fetch_project_config(proj_id).await?;

    cache.insert(proj_id, config.clone());

    Ok(config)
}

async fn fetch_project_config(proj_id: Uuid) -> Result<JsonValue> {
    let server_addr: String = crate::config::get("WATERWHEEL_SERVER_ADDR")?;

    let url = reqwest::Url::parse(&server_addr)?
        .join("api/projects/")?
        .join(&format!("{}/", proj_id))?
        .join("config")?;

    let client = reqwest::Client::new();

    trace!("fetching project config from api");

    let resp = client
        .get(url.clone())
        .send()
        .await?
        .error_for_status()?;

    println!("{:?}", resp);

    let config = resp.json().await?;

    trace!("got config");
    Ok(config)
}
