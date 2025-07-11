use crate::{
    messages::{TaskPriority, Token},
    server::{Server, execute::ExecuteToken},
    util::format_duration_approx,
};
use anyhow::{Result, format_err};
use binary_heap_plus::{BinaryHeap, MinComparator};
use chrono::{DateTime, Duration, Utc};
use postage::prelude::*;
use std::sync::Arc;
use tokio::select;
use tracing::{debug, info, trace, warn};
use uuid::Uuid;

type RetryQueue = BinaryHeap<Retry, MinComparator>;

#[derive(Clone, Debug)]
pub enum SubmitRetry {
    Add(Retry),
    Reload,
}

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Debug, sqlx::FromRow)]
pub struct Retry {
    // sort by time first
    pub retry_at_datetime: DateTime<Utc>,
    pub task_run_id: Uuid,
}

pub async fn process_retries(server: Arc<Server>) -> Result<!> {
    let mut retry_rx = server.post_office.receive_mail::<SubmitRetry>().await?;

    let mut queue = BinaryHeap::new_min();

    loop {
        trace!("checking if the queue is empty");
        if queue.is_empty() {
            trace!("retry queue is empty, waiting for new retries");
            let submit_retry = retry_rx
                .recv()
                .await
                .ok_or_else(|| format_err!("retry_rx channel was closed"))?;

            process_submit_retry(&server, &mut queue, submit_retry).await?;
            continue;
        }

        let next_retry = queue.pop().expect("queue is not empty now");

        let delay = next_retry.retry_at_datetime - Utc::now();

        info!(task_run_id=?next_retry.task_run_id,
                "sleeping {} until next retry", format_duration_approx(delay));

        if delay > Duration::zero() {
            select! {
                Some(submit_retry) = retry_rx.recv() => {
                    trace!("received a retry while sleeping");
                    // put the current retry back in the queue
                    queue.push(next_retry);
                    process_submit_retry(&server, &mut queue, submit_retry).await?;
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

async fn process_submit_retry(
    server: &Server,
    queue: &mut RetryQueue,
    submit_retry: SubmitRetry,
) -> Result<()> {
    match submit_retry {
        SubmitRetry::Add(retry) => {
            trace!("received a retry");
            queue.push(retry);
        }
        SubmitRetry::Reload => reload_retries(server, queue).await?,
    }
    Ok(())
}

#[derive(sqlx::FromRow)]
struct RetryInfo {
    pub task_id: Uuid,
    pub trigger_datetime: DateTime<Utc>,
    pub priority: TaskPriority,
    pub attempt: i64,
}

async fn do_retry(server: &Server, retry: Retry) -> Result<()> {
    let mut execute_tx = server.post_office.post_mail::<ExecuteToken>().await?;

    let info: RetryInfo = sqlx::query_as(
        "SELECT
            task_id,
            trigger_datetime,
            priority,
            attempt
        FROM task_run
        WHERE id = $1",
    )
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
        WHERE task_run_id = $1",
    )
    .bind(retry.task_run_id)
    .execute(&server.db_pool)
    .await?;

    Ok(())
}

async fn reload_retries(server: &Server, queue: &mut RetryQueue) -> Result<()> {
    debug!("loading retries from database");
    let retries = sqlx::query_as::<_, Retry>("SELECT retry_at_datetime, task_run_id FROM retry")
        .fetch_all(&server.db_pool)
        .await?;

    let me = &*server.node_id;
    let rendezvous = server.on_cluster_membership_change.borrow();

    for retry in retries {
        if rendezvous.item_is_mine(me, &retry.task_run_id) {
            queue.push(retry);
        }
    }
    debug!("loaded {} retries from database", queue.len());

    Ok(())
}

pub async fn retry_cluster_changes(server: Arc<Server>) -> Result<!> {
    let mut cluster_rx = server.on_cluster_membership_change.subscribe();
    //server.post_office.receive_mail::<ClusterMembershipChange>().await?;
    let mut retry_tx = server.post_office.post_mail::<SubmitRetry>().await?;

    loop {
        //let _ = cluster_rx.recv().await?;

        info!("cluster membership changed");
        retry_tx.send(SubmitRetry::Reload).await?;

        cluster_rx.changed().await?;
    }
}
