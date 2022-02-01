use crate::amqp;
use crate::messages::{ConfigUpdate, TaskDef};
use crate::server::jwt;
use anyhow::Result;
use futures::TryStreamExt;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::ExchangeKind;
use lru_time_cache::LruCache;
use once_cell::sync::Lazy;
use serde_json::Value as JsonValue;
use tokio::sync::Mutex;
use tracing::trace;
use uuid::Uuid;

const CONFIG_EXCHANGE: &str = "waterwheel.config";

static PROJ_CONFIG_CACHE: Lazy<Mutex<LruCache<Uuid, JsonValue>>> = Lazy::new(|| {
    Mutex::new(LruCache::with_expiry_duration_and_capacity(
        chrono::Duration::hours(24).to_std().unwrap(),
        100,
    ))
});

static TASK_DEF_CACHE: Lazy<Mutex<LruCache<Uuid, TaskDef>>> = Lazy::new(|| {
    Mutex::new(LruCache::with_expiry_duration_and_capacity(
        chrono::Duration::hours(24).to_std().unwrap(),
        100,
    ))
});

pub async fn get_project_config(proj_id: Uuid) -> Result<JsonValue> {
    let mut cache = PROJ_CONFIG_CACHE.lock().await;
    let config = cache.get(&proj_id);

    if let Some(config) = config {
        Ok(config.clone())
    } else {
        let config = fetch_project_config(proj_id).await?;
        cache.insert(proj_id, config.clone());
        Ok(config)
    }
}

pub async fn get_task_def(task_id: Uuid) -> Result<TaskDef> {
    let mut cache = TASK_DEF_CACHE.lock().await;
    let def = cache.get(&task_id);

    if let Some(def) = def {
        Ok(def.clone())
    } else {
        let def = fetch_task_def(task_id).await?;
        cache.insert(task_id, def.clone());
        Ok(def)
    }
}

async fn fetch_project_config(proj_id: Uuid) -> Result<JsonValue> {
    let server_addr = crate::config::get().server_addr.as_ref();

    let token = "Bearer ".to_owned() + &jwt::generate_config_jwt(proj_id)?;

    let url = reqwest::Url::parse(server_addr)?
        .join("int-api/projects/")?
        .join(&format!("{}/", proj_id))?
        .join("config")?;

    let client = reqwest::Client::new();

    trace!("fetching project config from api");

    let resp = client
        .get(url.clone())
        .header(reqwest::header::AUTHORIZATION, token)
        .send()
        .await?
        .error_for_status()?;

    let config = resp.json().await?;

    trace!("got config");
    Ok(config)
}

async fn fetch_task_def(task_id: Uuid) -> Result<TaskDef> {
    let server_addr = crate::config::get().server_addr.as_ref();

    let token = "Bearer ".to_owned() + &jwt::generate_config_jwt(task_id)?;

    let url = reqwest::Url::parse(server_addr)?
        .join("int-api/tasks/")?
        .join(&format!("{}", task_id))?;

    let client = reqwest::Client::new();

    trace!("fetching task def from api");

    let resp = client
        .get(url.clone())
        .header(reqwest::header::AUTHORIZATION, token)
        .send()
        .await?
        .error_for_status()?;
    let def = resp.json().await?;

    trace!("got task def");
    Ok(def)
}

pub async fn process_updates() -> Result<!> {
    let chan = amqp::get_amqp_channel().await?;

    // declare exchange for config updates
    chan.exchange_declare(
        CONFIG_EXCHANGE,
        ExchangeKind::Fanout,
        ExchangeDeclareOptions {
            durable: true,
            ..ExchangeDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    // declare queue for consuming incoming messages
    let queue = chan
        .queue_declare(
            "", // auto generate name on server side
            QueueDeclareOptions {
                durable: true,
                exclusive: true, // implies auto delete too
                ..QueueDeclareOptions::default()
            },
            FieldTable::default(),
        )
        .await?;

    // bind queue to the exchange
    chan.queue_bind(
        queue.name().as_str(),
        CONFIG_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )
    .await?;

    let mut consumer = chan
        .basic_consume(
            queue.name().as_str(),
            "worker",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some((chan, msg)) = consumer.try_next().await? {
        let update: ConfigUpdate = serde_json::from_slice(&msg.data)?;

        trace!("received config update message: {:?}", update);

        match update {
            ConfigUpdate::Project(proj_id) => drop_project_config(proj_id).await,
            ConfigUpdate::TaskDef(task_id) => drop_task_def(task_id).await,
        };

        chan.basic_ack(msg.delivery_tag, BasicAckOptions::default())
            .await?;

        trace!("updated config");
    }

    unreachable!("consumer stopped consuming")
}

pub async fn drop_project_config(proj_id: Uuid) {
    let mut cache = PROJ_CONFIG_CACHE.lock().await;
    cache.remove(&proj_id);
}

pub async fn drop_task_def(task_id: Uuid) {
    let mut cache = TASK_DEF_CACHE.lock().await;
    cache.remove(&task_id);
}
