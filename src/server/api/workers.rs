use crate::server::api::{State, auth, request_ext::RequestExt};
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow)]
struct WorkerState {
    pub uuid: Uuid,
    pub addr: String,
    pub version: String,
    pub last_seen_datetime: DateTime<Utc>,
    pub running_tasks: i32,
    pub total_tasks: i32,
    pub status: String,
}

pub async fn list(req: Request<State>) -> highnoon::Result<impl Responder> {
    auth::list().kind("workers").check(&req).await?;

    let workers: Vec<WorkerState> = sqlx::query_as(
        "SELECT
            id AS uuid,
            addr,
            version,
            last_seen_datetime,
            running_tasks,
            total_tasks,
            CASE
                WHEN CURRENT_TIMESTAMP - last_seen_datetime > INTERVAL '15 minutes' THEN 'gone'
                ELSE 'up'
            END AS status
        FROM worker w
        WHERE CURRENT_TIMESTAMP - last_seen_datetime < INTERVAL '24 hours'
        ORDER BY last_seen_datetime DESC",
    )
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(workers))
}

#[derive(Deserialize)]
struct QueryWorker {
    state: Option<String>,
}

#[derive(Serialize)]
struct GetWorker {
    last_seen_datetime: DateTime<Utc>,
    running_tasks: i32,
    total_tasks: i32,
    tasks: Vec<GetWorkerTask>,
    status: String,
    version: String,
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
    attempt: i64,
}

pub async fn tasks(req: Request<State>) -> highnoon::Result<Response> {
    let id = req.param("id")?.parse::<Uuid>()?;

    auth::get().kind("workers").check(&req).await?;

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
            r.task_id,
            r.trigger_datetime,
            queued_datetime,
            started_datetime,
            finish_datetime,
            r.state,
            r.attempt
        FROM task_run r
        JOIN task t ON t.id = r.task_id
        JOIN job j ON j.id = t.job_id
        JOIN project p ON p.id = j.project_id
        WHERE r.worker_id = $1
        AND ($2 IS NULL OR r.state = ANY($2))
        ORDER BY r.started_datetime DESC
        LIMIT 100",
    )
    .bind(id)
    .bind(&states)
    .fetch_all(&req.get_pool())
    .await?;

    let status: Option<WorkerState> = sqlx::query_as(
        "SELECT
            id AS uuid,
            addr,
            version,
            last_seen_datetime,
            running_tasks,
            total_tasks,
            CASE
                WHEN CURRENT_TIMESTAMP - last_seen_datetime > INTERVAL '15 minutes' THEN 'gone'
                ELSE 'up'
            END AS status
        FROM worker w
        WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&req.get_pool())
    .await?;

    if let Some(worker) = status {
        Response::ok().json(GetWorker {
            last_seen_datetime: worker.last_seen_datetime,
            running_tasks: worker.running_tasks,
            total_tasks: worker.total_tasks,
            tasks,
            status: worker.status,
            version: worker.version,
        })
    } else {
        Ok(Response::status(StatusCode::NOT_FOUND))
    }
}
