use std::sync::Arc;
use anyhow::{format_err, Result};
use binary_heap_plus::BinaryHeap;
use chrono::{DateTime, Duration, Utc};
use futures::TryStreamExt;
use postage::prelude::*;
use tokio::select;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;
use crate::messages::{TaskPriority, Token};
use crate::server::execute::ExecuteToken;
use crate::server::Server;
use crate::util::format_duration_approx;

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Debug, sqlx::FromRow)]
pub struct SubmitRetry {
    // sort by time first
    pub retry_at_datetime: DateTime<Utc>,
    pub task_run_id: Uuid,
}

pub async fn process_retries(server: Arc<Server>) -> Result<!> {
    let mut retry_rx = server.post_office.receive_mail::<SubmitRetry>().await?;

    let mut queue = BinaryHeap::new_min();

    debug!("restoring retries from database");
    let mut cursor = sqlx::query_as(
        "SELECT retry_at_datetime, task_run_id FROM retry"
    )
    .fetch(&server.db_pool);

    while let Some(retry) = cursor.try_next().await? {
        queue.push(retry);
    }
    debug!("restored {} retries from database", queue.len());

    loop {
        trace!("checking if the queue is empty");
        if queue.is_empty() {
            trace!("retry queue is empty, waiting for new retries");
            let retry = retry_rx
                .recv()
                .await
                .ok_or_else(|| format_err!("retry_rx channel was closed"))?;
            trace!("received a retry");
            queue.push(retry);
        }

        let next_retry = queue.pop().expect("queue is not empty now");

        let delay = next_retry.retry_at_datetime - Utc::now();

        info!(task_run_id=?next_retry.task_run_id,
                "sleeping {} until next retry", format_duration_approx(delay));

        if delay > Duration::zero() {
            select! {
                Some(new_retry) = retry_rx.recv() => {
                    trace!("received a retry while sleeping");
                    // put the current retry, and the new one in the queue
                    queue.push(next_retry);
                    queue.push(new_retry);
                }
                _ = tokio::time::sleep(delay.to_std()?) => {
                    trace!("sleep completed, no new retries");
                    do_retry(&server, next_retry).await?;
                }
            }
        } else {
            warn!("overslept retry: {}", delay);
            do_retry(&server, next_retry).await?;
        }
    }
}

#[derive(sqlx::FromRow)]
struct RetryInfo {
    pub task_id: Uuid,
    pub trigger_datetime: DateTime<Utc>,
    pub priority: TaskPriority,
    pub attempt: i64,
}

async fn do_retry(server: &Server, retry: SubmitRetry) -> Result<()> {
    let mut execute_tx = server.post_office.post_mail::<ExecuteToken>().await?;

    let info: RetryInfo = sqlx::query_as(
        "SELECT
            task_id,
            trigger_datetime,
            priority,
            attempt
        FROM task_run
        WHERE id = $1")
    .bind(retry.task_run_id)
    .fetch_one(&server.db_pool)
    .await?;

    info!(task_run_id=?retry.task_run_id,
        task_id=?info.task_id,
        trigger_datetime=?info.trigger_datetime,
        priority=?info.priority,
        attempt=?info.attempt,
        "retrying");

    execute_tx
        .send(ExecuteToken {
            token: Token {
                task_id: info.task_id,
                trigger_datetime: info.trigger_datetime,
            },
            priority: info.priority,
            attempt: u32::try_from(info.attempt)? + 1,
        })
        .await?;

    sqlx::query(
        "DELETE FROM retry
        WHERE task_run_id = $1")
    .bind(retry.task_run_id)
    .execute(&server.db_pool)
    .await?;

    Ok(())
}
