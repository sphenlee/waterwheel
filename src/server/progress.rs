use crate::amqp;
use crate::messages::{TaskPriority, TaskProgress, Token};
use crate::server::tokens::{increment_token, ProcessToken};
use crate::{db, postoffice};
use anyhow::Result;
use futures::TryStreamExt;
use kv_log_macro::{debug, info};
use lapin::options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use postage::prelude::*;
use sqlx::{types::Uuid, Connection, Postgres, Transaction};

const RESULT_QUEUE: &str = "waterwheel.results";

pub async fn process_progress() -> Result<!> {
    let chan = amqp::get_amqp_channel().await?;

    let mut token_tx = postoffice::post_mail::<ProcessToken>().await?;

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

        info!(
        "received task progress", {
            result: task_progress.result.to_string(),
            task_id: task_progress.task_id.to_string(),
            trigger_datetime: task_progress.trigger_datetime.to_rfc3339(),
        });

        let pool = db::get_pool();
        let mut conn = pool.acquire().await?;
        let mut txn = conn.begin().await?;

        let tokens_to_tx = if task_progress.result.is_final() {
            advance_tokens(&mut txn, &task_progress).await?
        } else {
            Vec::<Token>::new()
        };

        update_task_progress(&mut txn, &task_progress).await?;

        chan.basic_ack(msg.delivery_tag, BasicAckOptions::default())
            .await?;

        txn.commit().await?;

        debug!("finished processing task results");

        // after committing the transaction we can tell the token processor increment tokens
        for token in tokens_to_tx {
            token_tx
                .send(ProcessToken::Increment(token, TaskPriority::Normal))
                .await?;
        }
    }

    unreachable!("consumer stopped consuming")
}

pub async fn advance_tokens(
    txn: &mut Transaction<'_, Postgres>,
    task_progress: &TaskProgress,
) -> Result<Vec<Token>> {
    let pool = db::get_pool();

    let mut cursor = sqlx::query_as::<_, (Uuid,)>(
        "SELECT child_task_id
            FROM task_edge
            WHERE parent_task_id = $1
            AND kind = $2",
    )
    .bind(&task_progress.task_id)
    .bind(task_progress.result.to_string())
    .fetch(&pool);

    let mut tokens_to_tx = Vec::new();

    while let Some((child_task_id,)) = cursor.try_next().await? {
        let token = Token {
            task_id: child_task_id,
            trigger_datetime: task_progress.trigger_datetime,
        };

        increment_token(&mut *txn, &token).await?;
        tokens_to_tx.push(token);
    }

    Ok(tokens_to_tx)
}

async fn update_task_progress(
    txn: &mut Transaction<'_, Postgres>,
    task_progress: &TaskProgress,
) -> Result<()> {
    sqlx::query(
        "UPDATE token
            SET state = $1
        WHERE task_id = $2
        AND trigger_datetime = $3",
    )
    .bind(task_progress.result.to_string())
    .bind(&task_progress.task_id)
    .bind(&task_progress.trigger_datetime)
    .execute(&mut *txn)
    .await?;

    sqlx::query(
        "UPDATE task_run
            SET state = $1,
                started_datetime = $2,
                finish_datetime = $3,
                worker_id = $4
        WHERE id = $5",
    )
    .bind(task_progress.result.to_string())
    .bind(&task_progress.started_datetime)
    .bind(&task_progress.finished_datetime)
    .bind(&task_progress.worker_id)
    .bind(&task_progress.task_run_id)
    .execute(&mut *txn)
    .await?;

    Ok(())
}
