use crate::server::api::{App, AppResult, auth};
use axum::{
    extract::{Request, State},
    response::Json,
};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow)]
pub struct ServerStatus {
    pub num_projects: i64,
    pub num_workers: i64,
    pub running_tasks: i64,
}

#[axum::debug_handler]
pub async fn status(State(app): State<App>, req: Request) -> AppResult<Json<ServerStatus>> {
    auth::get(&app).kind("status").check(req.headers()).await?;

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
    .fetch_one(&app.get_pool())
    .await?;

    Ok(Json(status))
}
