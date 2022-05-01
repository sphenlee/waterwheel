use crate::{
    messages::{TaskProgress, Token},
    server::{
        tokens::{increment_token, ProcessToken},
        Server,
    },
};
use anyhow::Result;
use chrono::Duration;
use futures::TryStreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, BasicQosOptions, QueueDeclareOptions},
    types::FieldTable,
};
use postage::prelude::*;
use sqlx::{Connection, PgPool, Postgres, Transaction};
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;
use crate::messages::TaskPriority;

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

    while let Some((chan, msg)) = consumer.try_next().await? {
        let task_progress: TaskProgress = serde_json::from_slice(&msg.data)?;

        debug!(result=task_progress.result.as_ref(),
            task_id=?task_progress.task_id,
            trigger_datetime=?task_progress.trigger_datetime.to_rfc3339(),
            "received task progress");

        let mut conn = pool.acquire().await?;
        let mut txn = conn.begin().await?;

        let tokens_to_tx = if task_progress.result.is_final() {
            advance_tokens(&pool, &mut txn, &task_progress).await?
        } else {
            Vec::<Token>::new()
        };

        let priority = update_task_progress(&mut txn, &task_progress).await?;

        txn.commit().await?;

        chan.basic_ack(msg.delivery_tag, BasicAckOptions::default())
            .await?;

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
    let mut cursor = sqlx::query_as(
        "SELECT
            child_task_id,
            edge_offset
        FROM task_edge
        WHERE parent_task_id = $1
        AND kind = $2",
    )
    .bind(&task_progress.task_id)
    .bind(&task_progress.result)
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
    txn: &mut Transaction<'_, Postgres>,
    task_progress: &TaskProgress,
) -> Result<TaskPriority> {
    sqlx::query(
        "UPDATE token
            SET state = $1
        WHERE task_id = $2
        AND trigger_datetime = $3",
    )
    .bind(&task_progress.result)
    .bind(&task_progress.task_id)
    .bind(&task_progress.trigger_datetime)
    .execute(&mut *txn)
    .await?;

    let (priority,) = sqlx::query_as(
        "UPDATE task_run
            SET state = $1,
                started_datetime = $2,
                finish_datetime = $3,
                worker_id = $4
        WHERE id = $5
        RETURNING priority",
    )
    .bind(&task_progress.result)
    .bind(&task_progress.started_datetime)
    .bind(&task_progress.finished_datetime)
    .bind(&task_progress.worker_id)
    .bind(&task_progress.task_run_id)
    .fetch_one(&mut *txn)
    .await?;

    Ok(priority)
}
