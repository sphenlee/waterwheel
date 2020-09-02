use super::types::Job;
use super::util::RequestExt;
use super::State;
use super::{pg_error, PG_INTEGRITY_ERROR};
use crate::postoffice;
use crate::server::triggers::TriggerUpdate;
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
use hightide::{Json, Responder};
pub use tokens::{clear_tokens_trigger_datetime, get_tokens, get_tokens_trigger_datetime};
pub use triggers::{get_trigger, get_trigger_times, get_triggers_by_job};

pub async fn create(mut req: Request<State>) -> tide::Result<impl Responder> {
    let data = req.body_string().await?;
    let job: Job = serde_json::from_str(&data)?;

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
                raw_definition = $5
            WHERE id = $1",
        )
    } else {
        sqlx::query(
            "INSERT INTO job(id, name, project_id, description, raw_definition)
            VALUES($1, $2,
                (SELECT id FROM project WHERE name = $3),
                $4, $5)",
        )
    };

    let res = query
        .bind(&job.uuid)
        .bind(&job.name)
        .bind(&job.project)
        .bind(&job.description)
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
                Ok(StatusCode::Conflict)
            } else {
                Err(err.into())
            };
        }
    };

    let mut triggers_to_tx = Vec::new();

    // insert the triggers
    for trigger in &job.triggers {
        let id = triggers::create_trigger(&mut txn, &job, trigger).await?;
        triggers_to_tx.push(id);
    }

    for task in &job.tasks {
        tasks::create_task(&mut txn, task, &job).await?;
    }

    txn.commit().await?;

    for id in triggers_to_tx {
        trigger_tx.send(TriggerUpdate(id)).await;
    }

    Ok(StatusCode::Created)
}

#[derive(Deserialize)]
struct QueryJob {
    pub project: String,
    pub name: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct GetJob {
    pub id: Uuid,
    pub project: String,
    pub project_id: Uuid,
    pub name: String,
    pub description: String,
    pub raw_definition: String,
}

pub async fn get_by_name(req: Request<State>) -> tide::Result<impl Responder> {
    let q = req.query::<QueryJob>()?;

    let job = sqlx::query_as::<_, GetJob>(
        "SELECT
            j.id AS id,
            j.name AS name,
            p.name AS project,
            p.id AS project_id,
            j.description AS description,
            j.raw_definition AS raw_definition
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

pub async fn get_by_id(req: Request<State>) -> tide::Result<impl Responder> {
    let id = req.param::<Uuid>("id")?;

    let job = sqlx::query_as::<_, GetJob>(
        "SELECT
            j.id AS id,
            j.name AS name,
            p.name AS project,
            p.id AS project_id,
            j.description AS description,
            j.raw_definition AS raw_definition
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
            warn!("error deleting project: {}", err);
            Ok(StatusCode::InternalServerError)
        }
    }
}