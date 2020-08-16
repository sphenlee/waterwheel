use crate::trigger_time::TriggerTime;
use anyhow::Result;
use async_std::sync::Receiver;
use futures::StreamExt;
use sqlx::PgPool;
use log::debug;
use crate::amqp::amqp_connection;
use lapin::{ExchangeKind, BasicProperties};
use lapin::options::{ExchangeDeclareOptions, BasicPublishOptions, QueueDeclareOptions, QueueBindOptions};
use lapin::types::FieldTable;
use sqlx::types::Uuid;

const TASK_EXCHANGE: &str = "waterwheel.tasks";
const TASK_QUEUE: &str = "waterwheel.tasks";

#[derive(serde::Serialize)]
struct TaskDef {
    task_id: String,
}

pub async fn process_executions(_pool: PgPool, mut execute_rx: Receiver<Uuid>) -> Result<()> {
    let conn = amqp_connection().await?;
    let chan = conn.create_channel().await?;

    chan.exchange_declare(
        TASK_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions {
            durable: true,
            ..ExchangeDeclareOptions::default()
        },
        FieldTable::default()
    ).await?;

    chan.queue_declare(
        TASK_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        FieldTable::default()
    ).await?;

    chan.queue_bind(
        TASK_QUEUE,
        TASK_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    ).await?;

    // TODO - recover any tasks

    while let Some(task_id) = execute_rx.next().await {
        debug!("executing {}", task_id);

        chan.basic_publish(
            TASK_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            serde_json::to_vec(&TaskDef {
                task_id: task_id.to_string()
            })?,
            BasicProperties::default()
        ).await?;
    }

    Ok(())
}
