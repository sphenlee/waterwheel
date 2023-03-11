use crate::{
    messages::{ProcessToken, TaskPriority, TaskProgress, Token, TokenState},
    server::{tokens::increment_token, Server},
    util::first,
};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicQosOptions, QueueDeclareOptions},
    types::FieldTable,
};
use postage::prelude::*;
use sqlx::{Connection, PgPool, Postgres, Transaction};
use std::sync::Arc;
use tracing::{debug, info, trace};
use uuid::Uuid;
use crate::postoffice::PostOffice;
use crate::server::retries::{Retry, SubmitRetry};

const RESULT_QUEUE: &str = "waterwheel.results";

pub async fn process_progress(server: Arc<Server>) -> Result<!> {
    let pool = server.db_pool.clone();
    let chan = server.amqp_conn.create_channel().await?;

    let mut token_tx = server.post_office.post_mail::<ProcessToken>().await?;

    // declare queue for consuming incoming messages
    chan.queue_declare(
        RESULT_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    // to limit the number of redeliveries needed after a restart/crash
    chan.basic_qos(100, BasicQosOptions::default()).await?;

    let mut consumer = chan
        .basic_consume(
            RESULT_QUEUE,
            "server",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(delivery) = consumer.try_next().await? {
        let task_progress: TaskProgress = serde_json::from_slice(&delivery.data)?;

        debug!(result=task_progress.result.as_ref(),
            task_id=?task_progress.task_id,
            trigger_datetime=?task_progress.trigger_datetime.to_rfc3339(),
            "received task progress");

        let mut conn = pool.acquire().await?;
        let mut txn = conn.begin().await?;

        let priority = update_task_progress(&server, &mut txn, &task_progress).await?;

        let mut tokens_to_tx = Vec::new();

        if task_progress.result.is_final() {
            if task_progress.result.is_retryable()
                && has_retries(&pool, task_progress.task_run_id).await?
            {
                submit_retry(&server, &mut txn, &server.post_office, &task_progress).await?;
            } else {
                tokens_to_tx = advance_tokens(&pool, &mut txn, &task_progress).await?;
            }
        }

        txn.commit().await?;

        delivery.ack(BasicAckOptions::default()).await?;

        debug!("finished processing task results");

        // after committing the transaction we can tell the token processor increment tokens
        for token in tokens_to_tx {
            token_tx
                .send(ProcessToken::Increment(token, priority))
                .await?;
        }
    }

    unreachable!("consumer stopped consuming")
}

#[derive(sqlx::FromRow)]
struct TaskEdge {
    child_task_id: Uuid,
    edge_offset: Option<i64>,
}

pub async fn advance_tokens(
    pool: &PgPool,
    txn: &mut Transaction<'_, Postgres>,
    task_progress: &TaskProgress,
) -> Result<Vec<Token>> {
    trace!(task_id=?task_progress.task_id,
        task_run_id=?task_progress.task_run_id,
        "advancing tokens");

    let mut cursor = sqlx::query_as(
        "SELECT
            child_task_id,
            edge_offset
        FROM task_edge
        WHERE parent_task_id = $1
        AND kind = $2",
    )
    .bind(task_progress.task_id)
    .bind(task_progress.result)
    .fetch(pool);

    let mut tokens_to_tx = Vec::new();

    while let Some(TaskEdge {
        child_task_id,
        edge_offset,
    }) = cursor.try_next().await?
    {
        let token = Token {
            task_id: child_task_id,
            trigger_datetime: task_progress.trigger_datetime
                + Duration::seconds(edge_offset.unwrap_or(0)),
        };

        increment_token(&mut *txn, &token).await?;
        tokens_to_tx.push(token);
    }

    Ok(tokens_to_tx)
}

async fn update_task_progress(
    _server: &Server,
    txn: &mut Transaction<'_, Postgres>,
    task_progress: &TaskProgress,
) -> Result<TaskPriority> {
    trace!(task_id=?task_progress.task_id,
        task_run_id=?task_progress.task_run_id,
        "updating token state");

    sqlx::query(
        "UPDATE token
            SET state = $1
        WHERE task_id = $2
        AND trigger_datetime = $3",
    )
    .bind(task_progress.result)
    .bind(task_progress.task_id)
    .bind(task_progress.trigger_datetime)
    .execute(&mut *txn)
    .await?;

    trace!(task_id=?task_progress.task_id,
        task_run_id=?task_progress.task_run_id,
        "updating task_run state");

    let maybe_priority: Option<(TaskPriority,)> = sqlx::query_as(
        "UPDATE task_run
            SET state = $1,
                started_datetime = $2,
                finish_datetime = $3,
                updated_datetime = CURRENT_TIMESTAMP,
                worker_id = $4
        WHERE id = $5
        RETURNING priority",
    )
    .bind(task_progress.result)
    .bind(task_progress.started_datetime)
    .bind(task_progress.finished_datetime)
    .bind(task_progress.worker_id)
    .bind(task_progress.task_run_id)
    .fetch_optional(&mut *txn)
    .await?;

    // there are cases when the database doesn't record a task run for this UUID
    // (the message is sent to AMQP before the DB commits so we don't lose any events)
    // in that case we just keep going
    let priority = maybe_priority.map(first).unwrap_or_default();

    Ok(priority)
}

async fn has_retries(pool: &PgPool, task_run_id: Uuid) -> Result<bool> {
    trace!(?task_run_id, "checking if task has retries");

    let maybe_row: Option<(bool,)> = sqlx::query_as(
        "SELECT (r.attempt < t.retry_max_attempts) AS has_retries
        FROM task_run r
        JOIN task t ON r.task_id = t.id
        WHERE r.id = $1",
    )
    .bind(task_run_id)
    .fetch_optional(pool)
    .await?;

    Ok(maybe_row.map(first).unwrap_or(false))
}

async fn submit_retry(
    server: &Server,
    txn: &mut Transaction<'_, Postgres>,
    post_office: &PostOffice,
    task_progress: &TaskProgress
) -> Result<()> {
    debug!(task_id=?task_progress.task_id,
        task_run_id=?task_progress.task_run_id,
        "submitting retry");

    let (retry_at_datetime,): (DateTime<Utc>,) = sqlx::query_as(
        "SELECT $2 + (INTERVAL '1s' * COALESCE(t.retry_delay_secs, $3))
        FROM task t
        JOIN task_run r ON t.id = r.task_id
        WHERE r.id = $1",
    )
    .bind(task_progress.task_run_id)
    .bind(task_progress.finished_datetime.unwrap())
    .bind(server.config.default_task_retry_delay as i64)
    .fetch_one(&mut *txn)
    .await?;

    info!(task_id=?task_progress.task_id,
        task_run_id=?task_progress.task_run_id,
        "task will retry at {}", retry_at_datetime);

    sqlx::query(
        "INSERT INTO retry(task_run_id, retry_at_datetime)
        VALUES(
            $1,
            $2
        )",
    )
    .bind(task_progress.task_run_id)
    .bind(retry_at_datetime)
    .execute(&mut *txn)
    .await?;

    sqlx::query(
        "UPDATE token
            SET state = $1
        WHERE task_id = $2
        AND trigger_datetime = $3",
    )
    .bind(TokenState::Retry)
    .bind(task_progress.task_id)
    .bind(task_progress.trigger_datetime)
    .execute(&mut *txn)
    .await?;

    let mut retry_tx = post_office.post_mail::<SubmitRetry>().await?;
    retry_tx.send(SubmitRetry::Add(Retry {
        task_run_id: task_progress.task_run_id,
        retry_at_datetime,
    })).await?;

    Ok(())
}

