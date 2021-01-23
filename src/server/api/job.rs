use super::types::Job;
use super::request_ext::RequestExt;
use super::State;
use crate::postoffice;
use crate::server::triggers::TriggerUpdate;
use crate::util::{pg_error, PG_INTEGRITY_ERROR};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::Done;
use tide::{Request, StatusCode};
use uuid::Uuid;


mod graph;
mod tasks;
mod tokens;
mod triggers;

pub use graph::get_graph;
use hightide::{Json, Responder, Response};
pub use tokens::{
    clear_tokens_trigger_datetime, get_tokens, get_tokens_overview, get_tokens_trigger_datetime,
};
pub use triggers::{get_trigger, get_trigger_times, get_triggers_by_job};

pub async fn create(mut req: Request<State>) -> tide::Result<tide::Response> {
    let data = req.body_string().await?;
    let job: Job = match serde_json::from_str(&data) {
        Ok(json) => json,
        Err(err) => {
            return Response::status(StatusCode::UnprocessableEntity)
                .body(err.to_string())
                .into_response()
        }
    };

    let trigger_tx = postoffice::post_mail::<TriggerUpdate>().await?;

    let pool = req.get_pool();
    let mut txn = pool.begin().await?;

    let exists = sqlx::query("SELECT 1 FROM job WHERE id = $1")
        .bind(&job.uuid)
        .fetch_optional(&mut txn)
        .await?
        .is_some();

    let query = if exists {
        sqlx::query(
            "UPDATE job
            SET name = $2,
                project_id = (SELECT id FROM project WHERE name = $3),
                description = $4,
                paused = $5,
                raw_definition = $6
            WHERE id = $1",
        )
    } else {
        sqlx::query(
            "INSERT INTO job(id, name, project_id, description, paused, raw_definition)
            VALUES($1, $2,
                (SELECT id FROM project WHERE name = $3),
                $4, $5, $6)",
        )
    };

    let res = query
        .bind(&job.uuid)
        .bind(&job.name)
        .bind(&job.project)
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
            return if &err.code()[..2] == PG_INTEGRITY_ERROR {
                StatusCode::Conflict.into_response()
            } else {
                Err(err.into())
            };
        }
    };

    let mut triggers_to_tx = Vec::new();

    // insert the triggers
    for trigger in &job.triggers {
        match triggers::create_trigger(&mut txn, &job, trigger).await? {
            Ok(id) => triggers_to_tx.push(id),
            Err(err) => {
                return Response::status(StatusCode::BadRequest)
                    .body(err.to_string())
                    .into_response()
            }
        }
    }

    for task in &job.tasks {
        tasks::create_task(&mut txn, task, &job).await?;
    }

    txn.commit().await?;

    for id in triggers_to_tx {
        trigger_tx.send(TriggerUpdate(id)).await;
    }

    StatusCode::Created.into_response()
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

pub async fn get_by_name(req: Request<State>) -> tide::Result<impl Responder> {
    let q = req.query::<QueryJob>()?;

    let job = sqlx::query_as::<_, GetJob>(
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

    Ok(Json(job))
}

#[derive(Serialize, sqlx::FromRow)]
struct GetJobExtra {
    pub id: Uuid,
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

pub async fn get_by_id(req: Request<State>) -> tide::Result<impl Responder> {
    let id = req.param::<Uuid>("id")?;

    let job = sqlx::query_as::<_, GetJobExtra>(
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

    Ok(Json(job))
}

pub async fn delete(req: Request<State>) -> tide::Result<StatusCode> {
    let id = req.param::<Uuid>("id")?;

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
                Ok(StatusCode::NoContent)
            } else {
                info!("no job with id {}", id);
                Ok(StatusCode::NotFound)
            }
        }
        Err(err) => {
            warn!("error deleting job: {}", err);
            Ok(StatusCode::InternalServerError)
        }
    }
}


pub async fn get_paused(req: Request<State>) -> tide::Result<impl Responder> {
    let id = req.param::<Uuid>("id")?;

    let row = sqlx::query_as::<_, (bool,)>(
        "SELECT paused
        FROM job
        WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&req.get_pool())
    .await?;

    match row {
        Some((paused,)) => {
            Response::ok().json(paused)
        }
        None => {
            Ok(Response::status(StatusCode::NotFound))
        }
    }
}

#[derive(Deserialize)]
struct Paused {
    paused: bool,
}

pub async fn set_paused(mut req: Request<State>) -> impl Responder {
    let id = req.param::<Uuid>("id")?;

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
                return Ok(StatusCode::NotFound);
            }
        }
        Err(err) => {
            warn!("error pausing job: {}", err);
            return Ok(StatusCode::InternalServerError);
        }
    }

    // send trigger updates for the whole job to notify the scheduler
    let trigger_tx = postoffice::post_mail::<TriggerUpdate>().await?;

    let triggers_to_tx = sqlx::query_as::<_, (Uuid,)>(
        "SELECT id
        FROM trigger
        WHERE job_id = $1",
    )
    .bind(&id)
    .fetch_all(&req.get_pool())
    .await?;

    for (id,) in triggers_to_tx {
        trigger_tx.send(TriggerUpdate(id)).await;
    }

    Ok(StatusCode::NoContent)
}
