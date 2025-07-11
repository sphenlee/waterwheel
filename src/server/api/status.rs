use crate::server::api::{State, auth, request_ext::RequestExt};
use highnoon::{Json, Request, Responder};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow)]
pub struct ServerStatus {
    pub num_projects: i64,
    pub num_workers: i64,
    pub running_tasks: i64,
}

pub async fn status(req: Request<State>) -> highnoon::Result<impl Responder> {
    auth::get().kind("status").check(&req).await?;

    let status: ServerStatus = sqlx::query_as(
        "SELECT
            (
                SELECT COUNT(1)
                FROM project
            ) AS num_projects,
            (
                SELECT COUNT(1)
                FROM worker
                WHERE CURRENT_TIMESTAMP - last_seen_datetime < INTERVAL '15 minutes'
            ) AS num_workers,
            (
                SELECT COALESCE(SUM(running_tasks), 0)
                FROM worker
                WHERE CURRENT_TIMESTAMP - last_seen_datetime < INTERVAL '15 minutes'
            ) AS running_tasks",
    )
    .fetch_one(&req.get_pool())
    .await?;

    Ok(Json(status))
}
