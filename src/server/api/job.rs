use super::types::Job;
use super::util::{OptionExt, RequestExt};
use super::State;
use super::{pg_error, PG_INTEGRITY_ERROR};
use crate::postoffice;
use crate::server::triggers::TriggerUpdate;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::Done;
use tide::{Request, Response, StatusCode};
use uuid::Uuid;

mod tasks;
mod triggers;

pub async fn create(mut req: Request<State>) -> tide::Result<Response> {
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
                raw_definition = $4
            WHERE id = $1",
        )
    } else {
        sqlx::query(
            "INSERT INTO job(id, name, project_id, raw_definition)
            VALUES($1, $2,
                (SELECT id FROM project WHERE name = $3),
                $4)",
        )
    };

    let res = query
        .bind(&job.uuid)
        .bind(&job.name)
        .bind(&job.project)
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
                Ok(Response::from(StatusCode::Conflict))
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

    Ok(Response::from(StatusCode::Created))
}

#[derive(Deserialize)]
struct QueryJob {
    pub project: String,
    pub name: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct GetJob {
    pub uuid: Uuid,
    pub project: String,
    pub name: String,
    pub raw_definition: String,
}

pub async fn get_by_name(req: Request<State>) -> tide::Result<Response> {
    let q = req.query::<QueryJob>()?;

    let job = sqlx::query_as::<_, GetJob>(
        "SELECT
            j.id AS uuid,
            j.name AS name,
            p.name AS project,
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

    job.into_json_response()
}

pub async fn get_by_id(req: Request<State>) -> tide::Result<Response> {
    let id = req.param::<Uuid>("id")?;

    let job = sqlx::query_as::<_, GetJob>(
        "SELECT
            j.id AS uuid,
            j.name AS name,
            p.name AS project,
            j.raw_definition AS raw_definition
        FROM job j
        JOIN project p ON j.project_id = p.id
        WHERE j.id = $1",
    )
    .bind(&id)
    .fetch_optional(&req.get_pool())
    .await?;

    job.into_json_response()
}

pub async fn delete(req: Request<State>) -> tide::Result<StatusCode> {
    let id_str = req.param::<String>("id")?;
    let id = Uuid::parse_str(&id_str)?;

    let res = sqlx::query(
        "DELETE CASCADE FROM job
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
