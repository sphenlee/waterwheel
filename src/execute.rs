use crate::amqp::amqp_connection;
use crate::db;
use crate::tokens::Token;
use anyhow::Result;
use async_std::sync::Receiver;
use futures::StreamExt;
use lapin::options::{
    BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};
use log::{debug, info};

const TASK_EXCHANGE: &str = "waterwheel.tasks";
const TASK_QUEUE: &str = "waterwheel.tasks";

#[derive(serde::Serialize)]
struct TaskDef {
    task_id: String,
    trigger_datetime: String,
}

pub async fn process_executions(mut execute_rx: Receiver<Token>) -> Result<!> {
    let pool = db::get_pool();

    let conn = amqp_connection().await?;
    let chan = conn.create_channel().await?;

    chan.exchange_declare(
        TASK_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions {
            durable: true,
            ..ExchangeDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_declare(
        TASK_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_bind(
        TASK_QUEUE,
        TASK_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )
    .await?;

    // TODO - recover any tasks

    while let Some(token) = execute_rx.next().await {
        info!("enqueueing {}", token);

        chan.basic_publish(
            TASK_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            serde_json::to_vec(&TaskDef {
                task_id: token.task_id.to_string(),
                trigger_datetime: token.trigger_datetime.to_rfc3339(),
            })?,
            BasicProperties::default(),
        )
        .await?;

        sqlx::query(
            "UPDATE token
            SET state = 'active',
                count = count - (SELECT threshold FROM task WHERE id = task_id)
            WHERE task_id = $1
            AND trigger_datetime = $2",
        )
        .bind(token.task_id)
        .bind(token.trigger_datetime)
        .execute(&pool)
        .await?;

        debug!("done enqueueing {}", token);
    }

    unreachable!("execute_rx was closed")
}
