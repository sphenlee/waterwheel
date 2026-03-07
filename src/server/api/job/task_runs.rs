use crate::{
    messages::{TaskPriority, TokenState},
    server::api::{App, AppResult, auth},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct ListTaskRunsQuery {
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
#[axum::debug_handler]
pub async fn list_job_all_task_runs(
    State(app): State<App>,
    Path((job_id, trigger_datetime)): Path<(Uuid, DateTime<Utc>)>,
    Query(query): Query<ListTaskRunsQuery>,
    headers: HeaderMap,
) -> AppResult<Response> {
    auth::list(&app).job(job_id, None).check(&headers).await?;

    let tasks: Vec<ListJobAllTaskRuns> = sqlx::query_as(
        "SELECT
            tr.task_id AS task_id,
            tr.id AS task_run_id,
            t.name AS name,
            tr.trigger_datetime AS trigger_datetime,
            --rank() OVER (
            --    PARTITION BY tr.task_id
            --    ORDER BY tr.queued_datetime
            --) AS attempt,
            attempt,
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
        ORDER BY t.name ASC, tr.queued_datetime ASC
        LIMIT $3",
    )
    .bind(job_id)
    .bind(trigger_datetime)
    .bind(query.limit)
    .fetch_all(&app.get_pool())
    .await?;

    Ok(Json(tasks).into_response())
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

#[axum::debug_handler]
pub async fn list_task_runs(
    State(app): State<App>,
    Path((task_id, trigger_datetime)): Path<(Uuid, DateTime<Utc>)>,
    _headers: HeaderMap,
) -> AppResult<Response> {
    // TODO - auth via a task id?
    //auth::list(&app).job(job_id, None).check(&headers).await?;

    let tasks: Vec<ListTaskRuns> = sqlx::query_as(
        "SELECT
            tr.id AS task_run_id,
            --rank() OVER (
            --    ORDER BY tr.queued_datetime
            --) AS attempt,
            attempt,
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
    .bind(task_id)
    .bind(trigger_datetime)
    .fetch_all(&app.get_pool())
    .await?;

    Ok(Json(tasks).into_response())
}
