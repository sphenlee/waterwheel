use crate::{
    messages::ConfigUpdate,
    server::{
        api::{App, AppResult, auth, config_cache, types::Job, updates},
    },
    server::api::error::AppError,
    util::{is_pg_integrity_error, pg_error},
    messages::{ProcessToken, TriggerUpdate},
    util::first,
};
use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Response},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tracing::{info, warn};
use uuid::Uuid;

mod duration;
mod graph;
pub mod reference;
mod task_runs;
mod tasks;
mod tokens;
mod triggers;

pub use self::{
    duration::get_duration,
    graph::get_graph,
    tasks::list_tasks,
    tokens::{
        clear_tokens_trigger_datetime, get_tokens, get_tokens_overview, get_tokens_trigger_datetime,
    },
    triggers::{get_trigger, get_triggers_by_job},
    task_runs::{list_job_all_task_runs, list_task_runs}
};

pub async fn get_job_project_id(pool: &PgPool, job_id: Uuid) -> AppResult<Uuid> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT project_id
        FROM job
        WHERE id = $1",
    )
    .bind(job_id)
    .fetch_optional(pool)
    .await?;

    match row {
        None => Err(AppError::http((StatusCode::BAD_REQUEST, "job not found"))),
        Some((project_id,)) => Ok(project_id),
    }
}

/// resolve a project name into an ID
pub async fn get_project_id(pool: &PgPool, name: &str) -> AppResult<Uuid> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM project WHERE name = $1")
        .bind(name)
        .fetch_optional(pool)
        .await?;

    match row {
        None => Err(AppError::http((StatusCode::BAD_REQUEST, "project not found"))),
        Some((id,)) => Ok(id),
    }
}

#[axum::debug_handler]
pub async fn create(
    State(app): State<App>,
    headers: HeaderMap,
    Json(job): Json<Job>,
) -> AppResult<Response> {
    let pool = app.get_pool();

    let project_id = get_project_id(&pool, &job.project).await?;
    auth::update(&app).job(job.uuid, project_id).check(&headers).await?;

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
        .bind(job.uuid)
        .bind(&job.name)
        .bind(project_id)
        .bind(&job.description)
        .bind(job.paused)
        .bind(serde_json::to_string(&job)?)
        .execute(txn.as_mut())
        .await;

    match pg_error(res)? {
        Ok(_done) => {
            info!("created job {} -> {}", job.name, job.uuid);
        }
        Err(err) => {
            warn!("error creating job: {}", err);
            return if is_pg_integrity_error(&err) {
                Ok(StatusCode::CONFLICT.into_response())
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

    for task in &job.tasks {
        tasks::create_task_edges(&mut txn, task, &job).await?;
    }

    txn.commit().await?;

    updates::send_trigger_update(app.get_channel(), TriggerUpdate(triggers_to_tx)).await?;

    for id in tasks_to_tx {
        config_cache::send(app.get_channel(), ConfigUpdate::TaskDef(id)).await?;
    }

    Ok(StatusCode::CREATED.into_response())
}

#[derive(Deserialize)]
pub struct QueryJob {
    pub project: String,
    pub name: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct GetJob {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub description: String,
    pub paused: bool,
}

#[axum::debug_handler]
pub async fn get_by_name(
    State(app): State<App>,
    Query(q): Query<QueryJob>,
    headers: HeaderMap,
) -> AppResult<Response> {
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
    .fetch_optional(&app.get_pool())
    .await?;

    if let Some(job) = maybe_job {
        auth::get(&app).job(job.id, job.project_id).check(&headers).await?;
        Ok(Json(job).into_response())
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
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
    pub waiting_tasks: i64,
    pub failed_tasks_last_hour: i64,
    pub succeeded_tasks_last_hour: i64,
    pub error_tasks_last_hour: i64,
}

#[axum::debug_handler]
pub async fn get_by_id(
    State(app): State<App>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let maybe_job: Option<GetJobExtra> = sqlx::query_as(
        "WITH these_tasks AS (
            SELECT
                t.id AS id,
                tr.state AS state
            FROM task t
            JOIN task_run tr ON tr.task_id = t.id
            WHERE t.job_id = $1
            AND (finish_datetime IS NULL
                OR CURRENT_TIMESTAMP - finish_datetime < INTERVAL '1 hour')
         )
         SELECT
            j.id AS id,
            j.name AS name,
            p.name AS project,
            p.id AS project_id,
            j.description AS description,
            j.paused AS paused,
            j.raw_definition AS raw_definition,
            (
                SELECT COUNT(1)
                FROM these_tasks t
                WHERE (t.state = 'running')
            ) AS active_tasks,
            (
                SELECT COUNT(1)
                FROM these_tasks t
                WHERE (t.state = 'active' OR t.state = 'waiting')
            ) AS waiting_tasks,
            (
                SELECT COUNT(1)
                FROM these_tasks t
                WHERE t.state = 'failure'
            ) AS failed_tasks_last_hour,
            (
                SELECT COUNT(1)
                FROM these_tasks t
                WHERE t.state = 'success'
            ) AS succeeded_tasks_last_hour,
            (
                SELECT COUNT(1)
                FROM these_tasks t
                WHERE t.state = 'error'
            ) AS error_tasks_last_hour
        FROM job j
        JOIN project p ON j.project_id = p.id
        WHERE j.id = $1",
    )
    .bind(id)
    .fetch_optional(&app.get_pool())
    .await?;

    if let Some(job) = maybe_job {
        auth::get(&app).job(job.id, job.project_id).check(&headers).await?;
        Ok(Json(job).into_response())
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}

#[axum::debug_handler]
pub async fn delete(
    State(app): State<App>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    auth::delete(&app).job(id, None).check(&headers).await?;

    // TODO - this breaks because of foreign key constraints
    // should we even allow deleting a job?
    let res = sqlx::query(
        "DELETE FROM job
        WHERE id = $1",
    )
    .bind(id)
    .execute(&app.get_pool())
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

#[axum::debug_handler]
pub async fn get_paused(
    State(app): State<App>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let row: Option<(bool, Uuid)> = sqlx::query_as(
        "SELECT paused, project_id
        FROM job
        WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&app.get_pool())
    .await?;

    match row {
        Some((paused, proj_id)) => {
            auth::get(&app).job(id, proj_id).check(&headers).await?;
            Ok((StatusCode::OK, Json(paused)).into_response())
        }
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[derive(Deserialize)]
pub struct Paused {
    paused: bool,
}

#[axum::debug_handler]
pub async fn set_paused(
    State(app): State<App>,
    Path(job_id): Path<Uuid>,
    headers: HeaderMap,
    Json(Paused { paused }): Json<Paused>,
) -> AppResult<StatusCode> {
    auth::update(&app).job(job_id, None).check(&headers).await?;

    let row = sqlx::query(
        "UPDATE job
        SET paused = $2
        WHERE id = $1",
    )
    .bind(job_id)
    .bind(paused)
    .execute(&app.get_pool())
    .await;

    match row {
        Ok(done) => {
            if done.rows_affected() == 1 {
                if paused {
                    info!("paused job {}", job_id);
                } else {
                    info!("unpaused job {}", job_id);
                }
            } else {
                info!("no job with id {}", job_id);
                return Ok(StatusCode::NOT_FOUND);
            }
        }
        Err(err) => {
            warn!("error pausing job: {:?}", err);
            return Ok(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // send trigger updates for the whole job to notify the scheduler
    let triggers_to_tx: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id
        FROM trigger
        WHERE job_id = $1",
    )
    .bind(job_id)
    .fetch_all(&app.get_pool())
    .await?;

    let triggers_to_tx = triggers_to_tx.into_iter().map(first).collect();
    updates::send_trigger_update(app.get_channel(), TriggerUpdate(triggers_to_tx)).await?;

    // send taskdef updates for the whole job to notify the workers
    let tasks_to_tx: Vec<(Uuid,)> = sqlx::query_as(
        "SELECT id
        FROM task
        WHERE job_id = $1",
    )
    .bind(job_id)
    .fetch_all(&app.get_pool())
    .await?;

    for (id,) in tasks_to_tx {
        config_cache::send(app.get_channel(), ConfigUpdate::TaskDef(id)).await?;
    }

    // if job is being unpaused notify the token processor to trigger any pending tasks
    if !paused {
        updates::send_token_update(app.get_channel(), ProcessToken::UnpauseJob(job_id)).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}
