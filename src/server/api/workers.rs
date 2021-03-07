use crate::server::api::heartbeat::WORKER_STATUS;
use crate::server::api::request_ext::RequestExt;
use crate::server::api::State;
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub async fn list(_req: Request<State>) -> impl Responder {
    let status = WORKER_STATUS.lock().await;

    let workers: Vec<_> = status.values().cloned().collect();

    Json(workers)
}

#[derive(Deserialize)]
struct QueryWorker {
    state: Option<String>,
}

#[derive(Serialize)]
struct GetWorker {
    last_seen_datetime: DateTime<Utc>,
    running_tasks: u64,
    total_tasks: u64,
    tasks: Vec<GetWorkerTask>,
}

#[derive(Serialize, sqlx::FromRow)]
struct GetWorkerTask {
    job_id: Uuid,
    job_name: String,
    project_id: Uuid,
    project_name: String,
    task_run_id: Uuid,
    task_id: Uuid,
    task_name: String,
    trigger_datetime: DateTime<Utc>,
    queued_datetime: DateTime<Utc>,
    started_datetime: DateTime<Utc>,
    finish_datetime: Option<DateTime<Utc>>,
    state: String,
}

pub async fn tasks(req: Request<State>) -> highnoon::Result<Response> {
    let id = req.param("id")?.parse::<Uuid>()?;

    let q = req.query::<QueryWorker>()?;

    let states: Option<Vec<_>> = q.state.as_ref().map(|s| s.split(',').collect());

    let tasks: Vec<GetWorkerTask> = sqlx::query_as(
        "SELECT
            j.name AS job_name,
            j.id AS job_id,
            p.name AS project_name,
            p.id AS project_id,
            t.name AS task_name,
            r.id AS task_run_id,
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
        AND ($2 IS NULL OR r.state = ANY($2))
        ORDER BY r.started_datetime DESC
        LIMIT 100",
    )
    .bind(&id)
    .bind(&states)
    .fetch_all(&req.get_pool())
    .await?;

    let status = WORKER_STATUS.lock().await;
    if let Some(worker) = status.get(&id) {
        Response::ok().json(GetWorker {
            last_seen_datetime: worker.last_seen_datetime,
            running_tasks: worker.running_tasks,
            total_tasks: worker.total_tasks,
            tasks,
        })
    } else {
        Ok(Response::status(StatusCode::NOT_FOUND))
    }
}
