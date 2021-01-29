use crate::server::api::State;
use crate::server::api::request_ext::RequestExt;
use crate::server::heartbeat::WORKER_STATUS;
use highnoon::{Json, Responder, Request};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

pub async fn list(_req: Request<State>) -> impl Responder {
    let status = WORKER_STATUS.lock().await;

    let workers: Vec<_> = status.values().cloned().collect();

    Json(workers)
}

#[derive(Serialize, sqlx::FromRow)]
struct GetWorkerTask {
    job_id: Uuid,
    job_name: String,
    project_id: Uuid,
    project_name: String,
    task_id: Uuid,
    task_name: String,
    trigger_datetime: DateTime<Utc>,
    queued_datetime: DateTime<Utc>,
    started_datetime: DateTime<Utc>,
    finish_datetime: DateTime<Utc>,
    state: String,
}

pub async fn tasks(req: Request<State>) -> highnoon::Result<impl Responder> {
    let id = req.param("id")?.parse::<Uuid>()?;

    let tasks = sqlx::query_as::<_, GetWorkerTask>(
        "SELECT
            j.name AS job_name,
            j.id AS job_id,
            p.name AS project_name,
            p.id AS project_id,
            t.name AS task_name,
            r.task_id AS task_id,
            r.trigger_datetime AS trigger_datetime,
            queued_datetime,
            started_datetime,
            finish_datetime,
            r.state AS state
        FROM task_run r
        JOIN task t ON t.id = r.task_id
        JOIN job j ON j.id = t.job_id
        JOIN project p ON p.id = j.project_id
        WHERE r.worker_id = $1
        ORDER BY r.trigger_datetime DESC
        LIMIT 100",
    )
    .bind(&id)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(tasks))
}