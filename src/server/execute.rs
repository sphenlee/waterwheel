use crate::amqp::get_amqp_channel;
use crate::messages::{TaskPriority, TaskRequest, Token};
use crate::{db, postoffice, metrics};
use anyhow::Result;
use chrono::Utc;
use kv_log_macro::{debug as kvdebug, info as kvinfo};
use lapin::options::{
    BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};
use postage::prelude::*;
use sqlx::Connection;
use uuid::Uuid;
use cadence::Counted;

const TASK_EXCHANGE: &str = "waterwheel.tasks";
const TASK_QUEUE: &str = "waterwheel.tasks";

const PERSISTENT: u8 = 2;

#[derive(Debug, Clone)]
pub struct ExecuteToken(pub Token, pub TaskPriority);

pub async fn process_executions() -> Result<!> {
    let pool = db::get_pool();
    let statsd = metrics::get_client();

    let mut execute_rx = postoffice::receive_mail::<ExecuteToken>().await?;

    let chan = get_amqp_channel().await?;

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
    args.insert("x-max-priority".into(), 3.into());

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
        let ExecuteToken(token, priority) = msg;
        kvdebug!("enqueueing", {
            task_id: token.task_id.to_string(),
            trigger_datetime: token.trigger_datetime.to_rfc3339(),
            priority: log::kv::Value::from_debug(&priority),
        });

        let mut conn = pool.acquire().await?;
        let mut txn = conn.begin().await?;

        let task_req = TaskRequest {
            task_run_id: Uuid::new_v4(),
            task_id: token.task_id,
            trigger_datetime: token.trigger_datetime,
            priority,
        };

        let props = BasicProperties::default()
            .with_delivery_mode(PERSISTENT)
            .with_priority(priority as u8);

        chan.basic_publish(
            TASK_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            serde_json::to_vec(&task_req)?,
            props,
        )
        .await?;

        // TODO - check if the logic here is correct
        // used to set count = count - (SELECT threshold FROM task WHERE id = task_id)
        sqlx::query(
            "UPDATE token
            SET state = 'active',
                count = 0
            WHERE task_id = $1
            AND trigger_datetime = $2",
        )
        .bind(token.task_id)
        .bind(token.trigger_datetime)
        .execute(&mut txn)
        .await?;

        sqlx::query(
            "INSERT INTO task_run(id, task_id, trigger_datetime,
                queued_datetime, started_datetime, finish_datetime,
                worker_id, state)
            VALUES ($1, $2, $3,
                $4, NULL, NULL,
                NULL, 'active')",
        )
        .bind(&task_req.task_run_id)
        .bind(token.task_id)
        .bind(token.trigger_datetime)
        .bind(Utc::now())
        .execute(&mut txn)
        .await?;

        txn.commit().await?;

        kvinfo!("task enqueued", {
            task_id: token.task_id.to_string(),
            trigger_datetime: token.trigger_datetime.to_rfc3339(),
            priority: log::kv::Value::from_debug(&priority),
        });

        statsd.incr_with_tags("tasks.enqueued")
            .with_tag("priority", priority.as_str())
            .send();
    }

    unreachable!("ExecuteToken channel was closed!")
}
