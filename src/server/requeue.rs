use crate::{
    server::Server,
};
use anyhow::Result;
use futures::TryStreamExt;
use postage::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use chrono::{DateTime, Utc};
use tracing::{debug, warn};
use uuid::Uuid;
use crate::messages::{TaskPriority, Token, TokenState};
use crate::server::execute::ExecuteToken;

#[derive(sqlx::FromRow)]
struct Requeue {
    task_id: Uuid,
    trigger_datetime: DateTime<Utc>,
    priority: TaskPriority,
}

pub async fn process_requeue(server: Arc<Server>) -> Result<!> {
    let mut execute_tx = server.post_office.post_mail::<ExecuteToken>().await?;

    let mut ticker = tokio::time::interval(Duration::from_secs(server.config.requeue_interval_secs));

    loop {
        ticker.tick().await;
        debug!("checking for tasks to requeue");

        let mut cursor = sqlx::query_as::<_, Requeue>("
            UPDATE task_run
            SET state = $1,
                finish_datetime = CURRENT_TIMESTAMP
            WHERE state = $2
            AND updated_datetime < CURRENT_TIMESTAMP - INTERVAL '5 minutes'
            RETURNING task_id,
                      trigger_datetime,
                      priority
        ")
        .bind(TokenState::Error)
        .bind(TokenState::Running)
        .fetch(&server.db_pool);

        while let Some(requeue) = cursor.try_next().await? {
            warn!(task_id=?requeue.task_id,
                trigger_datetime=?requeue.trigger_datetime.to_rfc3339(),
                "requeueing task");

            execute_tx.send(ExecuteToken(
                Token {
                    task_id: requeue.task_id,
                    trigger_datetime: requeue.trigger_datetime,
                },
                requeue.priority,
            )).await?;

            sqlx::query("
                UPDATE token
                   SET state = $1
                 WHERE task_id = $2
                   AND trigger_datetime = $3
            ")
            .bind(TokenState::Running)
            .bind(requeue.task_id)
            .bind(requeue.trigger_datetime)
            .execute(&server.db_pool)
            .await?;
        }

        debug!("done checking for tasks to requeue");
    }
}

