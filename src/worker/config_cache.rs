use crate::{
    messages::{ConfigUpdate, TaskDef},
    server::jwt,
    worker::Worker,
};
use anyhow::Result;
use futures::TryStreamExt;
use highnoon::StatusCode;
use lapin::{
    options::{
        BasicAckOptions, BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions,
        QueueDeclareOptions,
    },
    types::FieldTable,
    ExchangeKind,
};
use serde_json::Value as JsonValue;
use std::{sync::Arc, time::Duration};
use tracing::{trace, warn};
use uuid::Uuid;

const CONFIG_EXCHANGE: &str = "waterwheel.config";

pub async fn get_project_config(worker: &Worker, proj_id: Uuid) -> Result<JsonValue> {
    let mut cache = worker.proj_config_cache.lock().await;
    let maybe_proj_config = cache.get(&proj_id);

    if let Some(proj_config) = maybe_proj_config {
        Ok(proj_config.clone())
    } else {
        let proj_config = fetch_project_config(&worker.config.server_addr, proj_id).await?;
        cache.insert(proj_id, proj_config.clone());
        Ok(proj_config)
    }
}

pub async fn get_task_def(worker: &Worker, task_id: Uuid) -> Result<Option<TaskDef>> {
    let mut cache = worker.task_def_cache.lock().await;
    let cache_def = cache.get(&task_id);

    if let Some(def) = cache_def {
        trace!(?task_id, "task def cache hit");
        Ok(def.clone())
    } else {
        let maybe_def = fetch_task_def(&worker.config.server_addr, task_id).await?;
        cache.insert(task_id, maybe_def.clone());
        Ok(maybe_def)
    }
}

async fn fetch_project_config(server_addr: &str, proj_id: Uuid) -> Result<JsonValue> {
    let token = "Bearer ".to_owned() + &jwt::generate_config_jwt(proj_id)?;

    let url = reqwest::Url::parse(server_addr)?
        .join("int-api/projects/")?
        .join(&format!("{}/", proj_id))?
        .join("config")?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    trace!(?proj_id, "fetching project config from api");

    let resp = client
        .get(url.clone())
        .header(reqwest::header::AUTHORIZATION, token)
        .send()
        .await?
        .error_for_status()?;

    let config = resp.json().await?;

    trace!(?proj_id, "got config");
    Ok(config)
}

async fn fetch_task_def(server_addr: &str, task_id: Uuid) -> Result<Option<TaskDef>> {
    let token = "Bearer ".to_owned() + &jwt::generate_config_jwt(task_id)?;

    let url = reqwest::Url::parse(server_addr)?
        .join("int-api/tasks/")?
        .join(&format!("{}", task_id))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;

    trace!(?task_id, "fetching task def from api");

    let res = client
        .get(url.clone())
        .header(reqwest::header::AUTHORIZATION, token)
        .send()
        .await;

    match res {
        Ok(resp) => match resp.status() {
            StatusCode::OK => {
                let def = resp.json().await?;
                trace!(?task_id, "got task def");
                Ok(Some(def))
            }
            StatusCode::NOT_FOUND => {
                trace!(?task_id, "task def not found");
                Ok(None)
            }
            otherwise => {
                warn!(
                    ?task_id,
                    "unexpected status code while fetching task_def: {}", otherwise
                );
                anyhow::bail!(
                    "unexpected status code while fetching task_def: {}",
                    otherwise
                );
            }
        },
        Err(err) => {
            warn!(?task_id, "error fetching task_def: {}", err);
            anyhow::bail!("error fetching task_def: {}", err);
        }
    }
}

pub async fn process_updates(worker: Arc<Worker>) -> Result<!> {
    let chan = worker.amqp_conn.create_channel().await?;

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
            ConfigUpdate::Project(proj_id) => drop_project_config(&worker, proj_id).await,
            ConfigUpdate::TaskDef(task_id) => drop_task_def(&worker, task_id).await,
        };

        chan.basic_ack(msg.delivery_tag, BasicAckOptions::default())
            .await?;

        trace!("updated config");
    }

    unreachable!("consumer stopped consuming")
}

pub async fn drop_project_config(worker: &Worker, proj_id: Uuid) {
    let mut cache = worker.proj_config_cache.lock().await;
    cache.remove(&proj_id);
}

pub async fn drop_task_def(worker: &Worker, task_id: Uuid) {
    let mut cache = worker.task_def_cache.lock().await;
    cache.remove(&task_id);
}
