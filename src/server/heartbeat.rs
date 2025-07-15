use crate::{GIT_VERSION, server::Server};
use anyhow::Result;
use sqlx::PgPool;
use std::sync::{Arc, atomic::Ordering};
use tracing::trace;

pub async fn post_heartbeat(server: &Server, pool: &PgPool) -> Result<()> {
    let waiting_for_trigger_id = *server.waiting_for_trigger_id.lock().await;

    sqlx::query(
        "
        INSERT INTO scheduler(
            id,
            last_seen_datetime,
            queued_triggers,
            waiting_for_trigger_id,
            version
        ) VALUES (
            $1,
            CURRENT_TIMESTAMP,
            $2,
            $3,
            $4
        )
        ON CONFLICT(id)
        DO UPDATE
        SET last_seen_datetime = CURRENT_TIMESTAMP,
            queued_triggers = $2,
            waiting_for_trigger_id = $3
        ",
    )
    .bind(server.scheduler_id)
    .bind(server.queued_triggers.load(Ordering::SeqCst) as i32)
    .bind(waiting_for_trigger_id)
    .bind(GIT_VERSION)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn heartbeat(server: Arc<Server>) -> Result<!> {
    let pool = server.db_pool.clone();

    loop {
        trace!("sending heartbeat");
        post_heartbeat(&server, &pool).await?;

        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
    }
}
