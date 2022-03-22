use crate::{
    messages::{TaskPriority, Token},
    server::{execute::ExecuteToken, Server},
};
use anyhow::Result;
use futures::TryStreamExt;
use postage::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, Transaction};
use std::{fmt, sync::Arc};
use tracing::{info, trace};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProcessToken {
    Increment(Token, TaskPriority),
    Activate(Token, TaskPriority),
    Clear(Token),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<token {} @ {}>",
            self.task_id,
            self.trigger_datetime.to_rfc3339()
        )
    }
}

async fn get_count_and_threshold(pool: &PgPool, token: &Token) -> Result<(i32, i32)> {
    let (count, threshold) = sqlx::query_as(
        "SELECT k.count, t.threshold
        FROM task t
        JOIN token k ON k.task_id = t.id
        WHERE t.id = $1
        AND k.trigger_datetime = $2",
    )
    .bind(&token.task_id)
    .bind(&token.trigger_datetime)
    .fetch_one(pool)
    .await?;

    Ok((count, threshold))
}

pub async fn process_tokens(server: Arc<Server>) -> Result<!> {
    let pool = server.db_pool.clone();

    restore_tokens(&server).await?;

    let mut token_rx = server.post_office.receive_mail::<ProcessToken>().await?;
    let mut execute_tx = server.post_office.post_mail::<ExecuteToken>().await?;

    while let Some(msg) = token_rx.recv().await {
        match msg {
            ProcessToken::Increment(token, priority) => {
                let (count, threshold) = get_count_and_threshold(&pool, &token).await?;

                trace!(task_id=?token.task_id,
                    trigger_datetime=?token.trigger_datetime.to_rfc3339(),
                    "count is {} (threshold {})", count, threshold);

                if count >= threshold {
                    execute_tx.send(ExecuteToken(token, priority)).await?;
                }
            }
            ProcessToken::Activate(token, priority) => {
                execute_tx.send(ExecuteToken(token, priority)).await?;
            }
            ProcessToken::Clear(_token) => {
                // TODO - don't need to know about token clears anymore
            }
        }
    }

    unreachable!("ProcessToken channel was closed!")
}

/// Adds a token to a task node
/// Usually you need to update some other state at the same time so it takes in a begun transaction
/// so you can do the other update before committing (eg. update the trigger's last trigger time, or
/// update the state of the upstream task to be 'done')
/// After adding the token you have to send the token over to the process_tokens future to actually
/// check if the node has activated
pub async fn increment_token(txn: &mut Transaction<'_, Postgres>, token: &Token) -> Result<()> {
    trace!(task_id=?token.task_id,
        trigger_datetime=?token.trigger_datetime.to_rfc3339(),
        "incrementing token");

    sqlx::query(
        "INSERT INTO token(task_id, trigger_datetime, count, state)
            VALUES ($1, $2, 1, 'waiting')
            ON CONFLICT(task_id, trigger_datetime)
            DO UPDATE SET count = token.count + 1",
    )
    .bind(token.task_id)
    .bind(token.trigger_datetime)
    .execute(&mut *txn)
    .await?;

    Ok(())
}

async fn restore_tokens(server: &Server) -> Result<()> {
    info!("restoring tokens from database...");

    let pool = server.db_pool.clone();

    let mut execute_tx = server.post_office.post_mail::<ExecuteToken>().await?;

    // first load all tokens from the DB
    let mut cursor = sqlx::query_as(
        "SELECT
            task.id,
            token.trigger_datetime
        FROM token
        JOIN task ON task.id = token.task_id
        WHERE token.count > task.threshold",
    )
    .fetch(&pool);

    let mut num_tokens_restored = 0;
    while let Some(row) = cursor.try_next().await? {
        let (task_id, trigger_datetime) = row;

        let token = Token {
            task_id,
            trigger_datetime,
        };

        trace!(task_id=?token.task_id,
            trigger_datetime=?token.trigger_datetime.to_rfc3339(),
            "restored token");

        execute_tx
            .send(ExecuteToken(token.clone(), TaskPriority::Normal))
            .await?;

        num_tokens_restored += 1;
    }

    info!(
        "done restoring {} tokens from database",
        num_tokens_restored
    );

    Ok(())
}
