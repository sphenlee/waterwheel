use crate::server::api::request_ext::RequestExt;
use crate::server::api::{auth, State};
use highnoon::{Json, Request, Responder};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow)]
pub struct ServerStatus {
    pub queued_triggers: i32,
    pub num_workers: i64,
    pub running_tasks: i64,
}

pub async fn status(req: Request<State>) -> highnoon::Result<impl Responder> {
    auth::get().kind("status").check(&req).await?;

    let status: ServerStatus = sqlx::query_as(
        "SELECT
            0 AS queued_triggers, -- TODO
            (
                SELECT COUNT(1)
                FROM worker
                WHERE CURRENT_TIMESTAMP - last_seen_datetime < INTERVAL '15 minutes'
            ) AS num_workers,
            COALESCE((
                SELECT SUM(running_tasks)
                FROM worker
                WHERE CURRENT_TIMESTAMP - last_seen_datetime < INTERVAL '15 minutes'
            ), 0) AS running_tasks",
    )
    .fetch_one(&req.get_pool())
    .await?;

    Ok(Json(status))
}
