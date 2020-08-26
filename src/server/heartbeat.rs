use crate::amqp::get_amqp_channel;
use crate::messages::WorkerHeartbeat;
use anyhow::Result;
use async_std::sync::Mutex;
use futures::TryStreamExt;
use kv_log_macro::debug;
use lapin::options::{
    BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::ExchangeKind;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use uuid::Uuid;

const HEARTBEAT_EXCHANGE: &str = "waterwheel.heartbeat";
const HEARTBEAT_QUEUE: &str = "waterwheel.heartbeat";

pub static WORKER_STATUS: Lazy<Mutex<HashMap<Uuid, WorkerHeartbeat>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn process_heartbeats() -> Result<!> {
    let chan = get_amqp_channel().await?;

    // declare queue for consuming incoming messages
    chan.queue_declare(
        HEARTBEAT_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    // declare exchange and bind to queue
    chan.exchange_declare(
        HEARTBEAT_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions {
            durable: false,
            ..ExchangeDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_bind(
        HEARTBEAT_QUEUE,
        HEARTBEAT_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )
    .await?;

    let mut consumer = chan
        .basic_consume(
            HEARTBEAT_QUEUE,
            "server",
            BasicConsumeOptions {
                no_ack: true,
                ..BasicConsumeOptions::default()
            },
            FieldTable::default(),
        )
        .await?;

    while let Some((_chan, msg)) = consumer.try_next().await? {
        let beat: WorkerHeartbeat = serde_json::from_slice(&msg.data)?;
        debug!("received heartbeat", {
            uuid: beat.uuid.to_string(),
        });

        WORKER_STATUS.lock().await.insert(beat.uuid, beat);
    }

    unreachable!("consumer stopped consuming")
}
