use crate::amqp::get_amqp_channel;
use crate::messages::WorkerHeartbeat;
use crate::server::status::SERVER_STATUS;
use anyhow::Result;
use futures::TryStreamExt;
use kv_log_macro::trace;
use lapin::options::{
    BasicConsumeOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::ExchangeKind;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::sync::Mutex;
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
        trace!("received heartbeat", {
            uuid: beat.uuid.to_string(),
        });

        let num_workers: usize;
        let running_tasks: u64;
        {
            let mut worker_status = WORKER_STATUS.lock().await;
            worker_status.insert(beat.uuid, beat);
            num_workers = worker_status.len();
            running_tasks = worker_status.values().map(|hb| hb.running_tasks).sum();
        }

        {
            let mut server_status = SERVER_STATUS.lock().await;
            server_status.num_workers = num_workers;
            server_status.running_tasks = running_tasks;
        }
    }

    unreachable!("consumer stopped consuming")
}
