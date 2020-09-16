use crate::amqp::get_amqp_channel;
use crate::messages::{TaskDef, TaskPriority, Token};
use crate::{db, postoffice};
use anyhow::Result;
use chrono::Utc;
use kv_log_macro::{debug, info};
use lapin::options::{
    BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};
use sqlx::Connection;
use uuid::Uuid;

const TASK_EXCHANGE: &str = "waterwheel.tasks";
const TASK_QUEUE: &str = "waterwheel.tasks";

const PERSISTENT: u8 = 2;

#[derive(Debug)]
pub struct ExecuteToken(pub Token, pub TaskPriority);

#[derive(sqlx::FromRow)]
struct TaskParams {
    task_name: String,
    job_id: Uuid,
    job_name: String,
    project_id: Uuid,
    project_name: String,
    image: Option<String>,
    args: Option<Vec<String>>,
    env: Option<Vec<String>>,
}

pub async fn process_executions() -> Result<!> {
    let pool = db::get_pool();

    let execute_rx = postoffice::receive_mail::<ExecuteToken>().await?;

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

    loop {
        let ExecuteToken(token, priority) = execute_rx.recv().await?;
        info!("enqueueing", {
            task_id: token.task_id.to_string(),
            trigger_datetime: token.trigger_datetime.to_rfc3339(),
            priority: log::kv::Value::from_debug(&priority),
        });

        let mut conn = pool.acquire().await?;
        let mut txn = conn.begin().await?;

        let params: TaskParams = sqlx::query_as(
            "SELECT
                t.name AS task_name,
                j.id AS job_id,
                j.name AS job_name,
                p.id AS project_id,
                p.name AS project_name,
                image,
                args,
                env
            FROM task t
            JOIN job j on t.job_id = j.id
            JOIN project p ON j.project_id = p.id
            WHERE t.id = $1",
        )
        .bind(&token.task_id)
        .fetch_one(&mut txn)
        .await?;

        let task_def = TaskDef {
            task_run_id: Uuid::new_v4(),
            task_id: token.task_id.clone(),
            task_name: params.task_name,
            job_id: params.job_id.clone(),
            job_name: params.job_name,
            project_id: params.project_id.clone(),
            project_name: params.project_name,
            trigger_datetime: token.trigger_datetime.clone(),
            image: params.image,
            args: params.args.unwrap_or_default(),
            env: params.env,
        };

        let props = BasicProperties::default()
            .with_delivery_mode(PERSISTENT)
            .with_priority(priority as u8);

        chan.basic_publish(
            TASK_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            serde_json::to_vec(&task_def)?,
            props,
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
        .execute(&mut txn)
        .await?;

        sqlx::query(
            "INSERT INTO task_run(id, task_id, trigger_datetime,
                queued_datetime, started_datetime, finish_datetime, state)
            VALUES ($1, $2, $3,
                $4, NULL, NULL, 'active')",
        )
        .bind(&task_def.task_run_id)
        .bind(token.task_id)
        .bind(token.trigger_datetime)
        .bind(Utc::now())
        .execute(&mut txn)
        .await?;

        txn.commit().await?;

        debug!("done enqueueing", {
            task_id: token.task_id.to_string(),
            trigger_datetime: token.trigger_datetime.to_rfc3339(),
        });
    }
}
/*
async fn mark_success(token: &Token) -> Result<()> {
    let pool = db::get_pool();
    let token_tx = postoffice::post_mail::<ProcessToken>().await?;

    let mut conn = pool.acquire().await?;
    let mut txn = conn.begin().await?;

    let task_result = TaskResult {
        task_id: token.task_id.to_string(),
        trigger_datetime: token.trigger_datetime.to_rfc3339(),
        result: "success".to_owned()
    };

    sqlx::query(
        "UPDATE token
                SET state = 'success',
                    count = count - (SELECT threshold FROM task WHERE id = task_id)
                WHERE task_id = $1
                AND trigger_datetime = $2",
    )
        .bind(token.task_id)
        .bind(token.trigger_datetime)
        .execute(&pool)
        .await?;

    let tokens_to_tx = advance_tokens(&mut txn, task_result).await?;

    txn.commit().await?;

    debug!("task ");

    // after committing the transaction we can tell the token processor to check thresholds
    for token in tokens_to_tx {
        token_tx.send(ProcessToken(token)).await;
    }

    Ok(())
}
*/
