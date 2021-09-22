use crate::util::{is_pg_integrity_error, pg_error};
use crate::messages::{ConfigUpdate, SchedulerUpdate};
use crate::server::api::{auth, config_cache, updates, State, types::Job, request_ext::RequestExt};
use crate::server::triggers::TriggerUpdate;
use highnoon::{Json, Request, Responder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;
use sqlx::PgPool;

mod graph;
mod tasks;
mod tokens;
mod triggers;

pub use self::graph::get_graph;
pub use self::tasks::list_tasks;
pub use self::tokens::{
    clear_tokens_trigger_datetime, get_tokens, get_tokens_overview, get_tokens_trigger_datetime,
};
pub use self::triggers::{get_trigger, get_trigger_times, get_triggers_by_job};

pub async fn get_job_project_id(
    pool: &PgPool,
    job_id: Uuid,
) -> highnoon::Result<Uuid> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT project_id
        FROM job
        WHERE id = $1",
    )
    .bind(&job_id)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Err(highnoon::Error::bad_request("job not found")),
        Some((project_id,)) => Ok(project_id),
    }
}

/// resolve a project name into an ID
pub async fn get_project_id(pool: &PgPool, name: &str) -> highnoon::Result<Uuid> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM project WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    match row {
        None => Err(highnoon::Error::bad_request("project not found")),
        Some((id,)) => Ok(id),
    }
}

pub async fn create(mut req: Request<State>) -> highnoon::Result<Response> {
    let pool = req.get_pool();

    let job: Job = req.body_json().await?;

    let project_id = get_project_id(&pool, &job.project).await?;
    auth::update().job(job.uuid, project_id).check(&req).await?;

    let mut txn = pool.begin().await?;

    let query = sqlx::query(
        "INSERT INTO job(
            id, name, project_id, description, paused, raw_definition
        ) VALUES (
            $1, $2, $3, $4,
            COALESCE($5, FALSE),
            $6
        )
        ON CONFLICT(id)
        DO UPDATE
        SET name = $2,
            project_id = $3,
            description = $4,
            paused = COALESCE($5, job.paused),
            raw_definition = $6",
    );

    let res = query
        .bind(&job.uuid)
        .bind(&job.name)
        .bind(&project_id)
        .bind(&job.description)
        .bind(&job.paused)
        .bind(serde_json::to_string(&job)?)
        .execute(&mut txn)
        .await;

    match pg_error(res)? {
        Ok(_done) => {
            info!("created job {} -> {}", job.name, job.uuid);
        }
        Err(err) => {
            warn!("error creating job: {}", err);
            return if is_pg_integrity_error(&err) {
                StatusCode::CONFLICT.into_response()
            } else {
                Err(err.into())
            };
        }
    };

    let mut triggers_to_tx = Vec::new();
    let mut tasks_to_tx = Vec::new();

    // insert the triggers
    for trigger in &job.triggers {
        let id = triggers::create_trigger(&mut txn, &job, trigger).await?;
        triggers_to_tx.push(id);
    }

    for task in &job.tasks {
        let id = tasks::create_task(&mut txn, task, &job).await?;
        tasks_to_tx.push(id);
    }

    txn.commit().await?;

    for id in triggers_to_tx {
        updates::send(
            req.get_channel(),
            SchedulerUpdate::TriggerUpdate(TriggerUpdate(id)),
        )
        .await?;
    }

    for id in tasks_to_tx {
        config_cache::send(req.get_channel(), ConfigUpdate::TaskDef(id)).await?;
    }

    StatusCode::CREATED.into_response()
}

#[derive(Deserialize)]
struct QueryJob {
    pub project: String,
    pub name: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct GetJob {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: String,
    pub paused: bool,
}

pub async fn get_by_name(req: Request<State>) -> highnoon::Result<impl Responder> {
    let q = req.query::<QueryJob>()?;

    let maybe_job: Option<GetJob> = sqlx::query_as(
        "SELECT
            j.id AS id,
            j.name AS name,
            j.project_id AS project_id,
            j.description AS description,
            j.paused AS paused
        FROM job j
        JOIN project p ON j.project_id = p.id
        WHERE j.name = $1
        AND p.name = $2",
    )
    .bind(&q.name)
    .bind(&q.project)
    .fetch_optional(&req.get_pool())
    .await?;

    if let Some(job) = maybe_job {
        auth::get().job(job.id, job.project_id).check(&req).await?;
        Json(job).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

#[derive(Serialize, sqlx::FromRow)]
struct GetJobExtra {
    pub id: Uuid, // TODO - consistency in naming ids
    pub project: String,
    pub project_id: Uuid,
    pub name: String,
    pub description: String,
    pub paused: bool,
    pub raw_definition: String,
    pub active_tasks: i64,
    pub failed_tasks_last_hour: i64,
    pub succeeded_tasks_last_hour: i64,
}

pub async fn get_by_id(req: Request<State>) -> highnoon::Result<impl Responder> {
    let id = req.param("id")?.parse::<Uuid>()?;

    let maybe_job: Option<GetJobExtra> = sqlx::query_as(
        "SELECT
            j.id AS id,
            j.name AS name,
            p.name AS project,
            p.id AS project_id,
            j.description AS description,
            j.paused AS paused,
            j.raw_definition AS raw_definition,
            (
                SELECT count(1)
                FROM task t
                JOIN task_run tr ON tr.task_id = t.id
                WHERE t.job_id = $1
                AND tr.state = 'active'
            ) AS active_tasks,
            (
                SELECT count(1)
                FROM task t
                JOIN task_run tr ON tr.task_id = t.id
                WHERE t.job_id = $1
                AND tr.state = 'failure'
                AND CURRENT_TIMESTAMP - finish_datetime < INTERVAL '1 hour'
            ) AS failed_tasks_last_hour,
            (
                SELECT count(1)
                FROM task t
                JOIN task_run tr ON tr.task_id = t.id
                WHERE t.job_id = $1
                AND tr.state = 'success'
                AND CURRENT_TIMESTAMP - finish_datetime < INTERVAL '1 hour'
            ) AS succeeded_tasks_last_hour
        FROM job j
        JOIN project p ON j.project_id = p.id
        WHERE j.id = $1",
    )
    .bind(&id)
    .fetch_optional(&req.get_pool())
    .await?;

    if let Some(job) = maybe_job {
        auth::get().job(job.id, job.project_id).check(&req).await?;
        Json(job).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn delete(req: Request<State>) -> highnoon::Result<StatusCode> {
    let id = req.param("id")?.parse::<Uuid>()?;

    auth::delete().job(id, None).check(&req).await?;

    // TODO - this breaks because of foreign key constraints
    // should we even allow deleting a job?
    let res = sqlx::query(
        "DELETE FROM job
        WHERE id = $1",
    )
    .bind(&id)
    .execute(&req.get_pool())
    .await;

    match pg_error(res)? {
        Ok(done) => {
            if done.rows_affected() == 1 {
                info!("deleted job {}", id);
                Ok(StatusCode::NO_CONTENT)
            } else {
                info!("no job with id {}", id);
                Ok(StatusCode::NOT_FOUND)
            }
        }
        Err(err) => {
            warn!("error deleting job: {}", err);
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_paused(req: Request<State>) -> highnoon::Result<impl Responder> {
    let id = req.param("id")?.parse::<Uuid>()?;

    let row: Option<(bool, Uuid)> = sqlx::query_as(
        "SELECT paused, project_id
        FROM job
        WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&req.get_pool())
    .await?;

    match row {
        Some((paused, proj_id)) => {
            auth::get().job(id, proj_id).check(&req).await?;
            Response::ok().json(paused)
        },
        None => Ok(Response::status(StatusCode::NOT_FOUND)),
    }
}

#[derive(Deserialize)]
struct Paused {
    paused: bool,
}

pub async fn set_paused(mut req: Request<State>) -> impl Responder {
    let id = req.param("id")?.parse::<Uuid>()?;

    auth::update().job(id, None).check(&req).await?;

    let Paused { paused } = req.body_json().await?;

    let row = sqlx::query(
        "UPDATE job
        SET paused = $2
        WHERE id = $1",
    )
    .bind(&id)
    .bind(&paused)
    .execute(&req.get_pool())
    .await;

    match row {
        Ok(done) => {
            if done.rows_affected() == 1 {
                if paused {
                    info!("paused job {}", id);
                } else {
                    info!("unpaused job {}", id);
                }
            } else {
                info!("no job with id {}", id);
                return Ok(StatusCode::NOT_FOUND);
            }
        }
        Err(err) => {
            warn!("error pausing job: {}", err);
            return Ok(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // send trigger updates for the whole job to notify the scheduler
    let triggers_to_tx = sqlx::query_as(
        "SELECT id
        FROM trigger
        WHERE job_id = $1",
    )
    .bind(&id)
    .fetch_all(&req.get_pool())
    .await?;

    for (id,) in triggers_to_tx {
        updates::send(
            req.get_channel(),
            SchedulerUpdate::TriggerUpdate(TriggerUpdate(id)),
        )
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}
