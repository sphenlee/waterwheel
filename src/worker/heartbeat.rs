use crate::amqp::get_amqp_channel;
use crate::messages::WorkerHeartbeat;
use anyhow::Result;
use std::net::SocketAddr;

use chrono::Utc;
use kv_log_macro::trace;
use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};

use super::{RUNNING_TASKS, TOTAL_TASKS, WORKER_ID};
use std::sync::atomic::Ordering;

const HEARTBEAT_EXCHANGE: &str = "waterwheel.heartbeat";

pub async fn heartbeat(addr: SocketAddr) -> Result<!> {
    let chan = get_amqp_channel().await?;

    // declare outgoing exchange
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

    loop {
        trace!("posting heartbeat");
        chan.basic_publish(
            HEARTBEAT_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            serde_json::to_vec(&WorkerHeartbeat {
                uuid: *WORKER_ID,
                addr: addr.to_string(),
                last_seen_datetime: Utc::now(),
                running_tasks: RUNNING_TASKS.load(Ordering::Relaxed),
                total_tasks: TOTAL_TASKS.load(Ordering::Relaxed),
            })?,
            BasicProperties::default(),
        )
        .await?;

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
