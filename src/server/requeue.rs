use crate::{
    messages::{TaskPriority, Token, TokenState},
    server::{execute::ExecuteToken, Server},
};
use anyhow::{format_err, Result};
use chrono::{DateTime, Utc};
use postage::prelude::*;
use sqlx::postgres::types::PgInterval;
use std::{sync::Arc, time::Duration};
use tracing::{debug, warn};
use uuid::Uuid;

#[derive(sqlx::FromRow)]
struct Requeue {
    task_run_id: Uuid,
    task_id: Uuid,
    trigger_datetime: DateTime<Utc>,
    priority: TaskPriority,
    attempt: i64,
    paused: bool,
}

pub async fn process_requeue(server: Arc<Server>) -> Result<!> {
    let mut execute_tx = server.post_office.post_mail::<ExecuteToken>().await?;

    let timeout: PgInterval = (Duration::from_secs(server.config.task_heartbeat)
        * server.config.requeue_missed_heartbeats)
        .try_into()
        .map_err(|err| format_err!("error converting duration to pg_interval: {:?}", err))?;

    let mut ticker =
        tokio::time::interval(Duration::from_secs(server.config.requeue_interval));

    ticker.tick().await; // first tick happens immediately

    loop {
        ticker.tick().await;
        debug!("checking for tasks to requeue");

        let mut txn = server.db_pool.begin().await?;

        let requeues = sqlx::query_as::<_, Requeue>(
            "SELECT
                r.id AS task_run_id,
                r.task_id,
                r.trigger_datetime,
                r.priority,
                r.attempt,
                j.paused
            FROM task_run r
            JOIN task t ON r.task_id = t.id
            JOIN job j ON t.job_id = j.id
            WHERE (
                r.state = $1
            OR
               (NOT j.paused AND r.state = $2)
            )
            AND r.updated_datetime < CURRENT_TIMESTAMP - $3
            FOR UPDATE OF r",
        )
        .bind(TokenState::Running)
        .bind(TokenState::Cancelled)
        .bind(timeout)
        .fetch_all(txn.as_mut())
        .await?;

        for requeue in requeues {
            if requeue.paused {
                warn!(task_run_id=?requeue.task_run_id,
                    task_id=?requeue.task_id,
                    trigger_datetime=?requeue.trigger_datetime.to_rfc3339(),
                    "cancelling running task for paused job");
            } else {
                warn!(task_run_id=?requeue.task_run_id,
                    task_id=?requeue.task_id,
                    trigger_datetime=?requeue.trigger_datetime.to_rfc3339(),
                    "requeueing task");

                execute_tx
                    .send(ExecuteToken {
                        token: Token {
                            task_id: requeue.task_id,
                            trigger_datetime: requeue.trigger_datetime,
                        },
                        priority: requeue.priority,
                        attempt: u32::try_from(requeue.attempt)? + 1,
                    })
                    .await?;
            }

            sqlx::query(
                "UPDATE task_run
                SET state = $1,
                    finish_datetime = CURRENT_TIMESTAMP
                WHERE id = $2",
            )
            .bind(TokenState::Error)
            .bind(requeue.task_run_id)
            .execute(txn.as_mut())
            .await?;

            sqlx::query(
                "UPDATE token
                   SET state = $1
                 WHERE task_id = $2
                   AND trigger_datetime = $3",
            )
            .bind(TokenState::Error)
            .bind(requeue.task_id)
            .bind(requeue.trigger_datetime)
            .execute(txn.as_mut())
            .await?;
        }

        txn.commit().await?;

        debug!("done checking for tasks to requeue");
    }
}
