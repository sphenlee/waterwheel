use crate::{
    messages::{TaskPriority, TaskRequest, Token},
    server::Server,
};
use anyhow::Result;
use cadence::CountedExt;
use chrono::Utc;
use lapin::{
    BasicProperties, ExchangeKind,
    options::{BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
};
use postage::prelude::*;
use sqlx::Connection;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

const TASK_EXCHANGE: &str = "waterwheel.tasks";
const TASK_QUEUE: &str = "waterwheel.tasks";

const PERSISTENT: u8 = 2;

#[derive(Debug, Clone)]
pub struct ExecuteToken {
    pub token: Token,
    pub priority: TaskPriority,
    pub attempt: u32,
}

pub async fn process_executions(server: Arc<Server>) -> Result<!> {
    let pool = server.db_pool.clone();
    let statsd = server.statsd.clone();

    let mut execute_rx = server.post_office.receive_mail::<ExecuteToken>().await?;

    let chan = server.amqp_conn.create_channel().await?;

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

    let mut args = FieldTable::default();
    args.insert("x-max-priority".into(), 3i8.into());

    let timeout_ms = server.config.amqp_consumer_timeout * 1000;
    args.insert("x-consumer-timeout".into(), timeout_ms.into());

    chan.queue_declare(
        TASK_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        args,
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

    while let Some(msg) = execute_rx.recv().await {
        let ExecuteToken {
            token,
            priority,
            attempt,
        } = msg;

        debug!(task_id=?token.task_id,
            trigger_datetime=%token.trigger_datetime.to_rfc3339(),
            ?priority,
            ?attempt,
            "enqueueing");

        let mut conn = pool.acquire().await?;
        let mut txn = conn.begin().await?;

        let task_req = TaskRequest {
            task_run_id: Uuid::new_v4(),
            task_id: token.task_id,
            trigger_datetime: token.trigger_datetime,
        };

        let props = BasicProperties::default()
            .with_delivery_mode(PERSISTENT)
            .with_priority(priority as u8);

        chan.basic_publish(
            TASK_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            &serde_json::to_vec(&task_req)?,
            props,
        )
        .await?;

        sqlx::query(
            "UPDATE token
            SET state = 'active',
                count = count - (SELECT threshold FROM task WHERE id = $1)
            WHERE task_id = $1
            AND trigger_datetime = $2",
        )
        .bind(token.task_id)
        .bind(token.trigger_datetime)
        .execute(txn.as_mut())
        .await?;

        sqlx::query(
            "INSERT INTO task_run(id, task_id, trigger_datetime,
                queued_datetime, started_datetime, finish_datetime,
                updated_datetime,
                worker_id, state, priority, attempt)
            VALUES ($1, $2, $3,
                $4, NULL, NULL,
                NULL,
                NULL, 'active', $5, $6)",
        )
        .bind(task_req.task_run_id)
        .bind(token.task_id)
        .bind(token.trigger_datetime)
        .bind(Utc::now())
        .bind(priority)
        .bind(attempt as i64)
        .execute(txn.as_mut())
        .await?;

        txn.commit().await?;

        info!(task_id=?token.task_id,
            trigger_datetime=%token.trigger_datetime.to_rfc3339(),
            ?priority,
            ?attempt,
            "task enqueued");

        statsd
            .incr_with_tags("tasks.enqueued")
            .with_tag("priority", priority.as_str())
            .send();
    }

    unreachable!("ExecuteToken channel was closed!")
}
