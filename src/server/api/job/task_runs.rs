use crate::{
    messages::{TaskPriority, TokenState},
    server::api::{auth, request_ext::RequestExt, State},
};
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
struct ListTaskRunsQuery {
    limit: Option<i32>,
}

#[derive(Serialize, sqlx::FromRow)]
struct ListJobAllTaskRuns {
    task_id: Uuid,
    task_run_id: Uuid,
    name: String,
    trigger_datetime: DateTime<Utc>,
    attempt: i64,
    queued_datetime: Option<DateTime<Utc>>,
    started_datetime: Option<DateTime<Utc>>,
    finish_datetime: Option<DateTime<Utc>>,
    state: TokenState,
    priority: TaskPriority,
    worker_id: Option<Uuid>,
}
pub async fn list_job_all_task_runs(req: Request<State>) -> highnoon::Result<impl Responder> {
    let job_id: Uuid = req.param("id")?.parse()?;
    let trigger_datetime: DateTime<Utc> = req.param("trigger_datetime")?.parse()?;
    let query: ListTaskRunsQuery = req.query()?;

    auth::list().job(job_id, None).check(&req).await?;

    let tasks: Vec<ListJobAllTaskRuns> = sqlx::query_as(
        "SELECT
            tr.task_id AS task_id,
            tr.id AS task_run_id,
            t.name AS name,
            tr.trigger_datetime AS trigger_datetime,
            rank() OVER (
                PARTITION BY tr.task_id
                ORDER BY tr.queued_datetime
            ) AS attempt,
            queued_datetime,
            started_datetime,
            finish_datetime,
            state,
            priority,
            worker_id
        FROM task_run tr
        JOIN task t ON t.id = tr.task_id
        WHERE t.job_id = $1
        AND tr.trigger_datetime = $2
        ORDER BY t.name
        LIMIT $3",
    )
    .bind(&job_id)
    .bind(&trigger_datetime)
    .bind(&query.limit)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(tasks))
}

#[derive(Serialize, sqlx::FromRow)]
struct ListTaskRuns {
    task_run_id: Uuid,
    attempt: i64,
    queued_datetime: Option<DateTime<Utc>>,
    started_datetime: Option<DateTime<Utc>>,
    finish_datetime: Option<DateTime<Utc>>,
    state: TokenState,
    priority: TaskPriority,
    worker_id: Option<Uuid>,
}

pub async fn list_task_runs(req: Request<State>) -> highnoon::Result<impl Responder> {
    let task_id: Uuid = req.param("id")?.parse()?;
    let trigger_datetime: DateTime<Utc> = req.param("trigger_datetime")?.parse()?;

    // TODO - auth via a task id?
    //auth::list().job(job_id, None).check(&req).await?;

    let tasks: Vec<ListTaskRuns> = sqlx::query_as(
        "SELECT
            tr.id AS task_run_id,
            rank() OVER (
                ORDER BY tr.queued_datetime
            ) AS attempt,
            queued_datetime,
            started_datetime,
            finish_datetime,
            state,
            priority,
            worker_id
        FROM task_run tr
        JOIN task t ON t.id = tr.task_id
        WHERE tr.task_id = $1
        AND tr.trigger_datetime = $2
        ORDER BY queued_datetime",
    )
    .bind(&task_id)
    .bind(&trigger_datetime)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(tasks))
}
