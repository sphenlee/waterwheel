use crate::server::api::request_ext::RequestExt;
use crate::server::api::State;
use highnoon::{Json, Request, Responder};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow)]
pub struct ServerStatus {
    pub queued_triggers: i32,
    pub num_workers: i64,
    pub running_tasks: i64,
}

pub async fn status(req: Request<State>) -> highnoon::Result<impl Responder> {
    let status: ServerStatus = sqlx::query_as(
        "SELECT
            0 AS queued_triggers, -- TODO
            (
                SELECT COUNT(1)
                FROM worker
            ) AS num_workers,
            (
                SELECT SUM(running_tasks)
                FROM worker
            ) AS running_tasks",
    )
    .fetch_one(&req.get_pool())
    .await?;

    Ok(Json(status))
}
