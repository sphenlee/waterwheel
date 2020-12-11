use super::util::RequestExt;
use super::{pg_error, State, PG_INTEGRITY_ERROR};
use hightide::{Json, Responder, Response};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::Done;
use tide::{Request, StatusCode};
use uuid::Uuid;

#[derive(Deserialize)]
struct NewProject {
    pub name: String,
    pub description: String,
}

pub async fn create(mut req: Request<State>) -> tide::Result<Response> {
    let proj: NewProject = req.body_json().await?;

    let id = uuid::Uuid::new_v4();

    let res = sqlx::query(
        "INSERT INTO project(id, name, description)
        VALUES($1, $2, $3)",
    )
    .bind(&id)
    .bind(&proj.name)
    .bind(&proj.description)
    .execute(&req.get_pool())
    .await;

    match pg_error(res)? {
        Ok(_done) => {
            info!("created project {} -> {}", proj.name, id);
            let proj = Project {
                id,
                name: proj.name,
                description: proj.description,
            };
            Ok(Response::status(StatusCode::Created).json(proj)?)
        }
        Err(err) => {
            warn!("error creating project: {}", err);
            if &err.code()[..2] == PG_INTEGRITY_ERROR {
                Ok(Response::status(StatusCode::Conflict))
            } else {
                Ok(Response::status(StatusCode::InternalServerError))
            }
        }
    }
}

pub async fn update(mut req: Request<State>) -> tide::Result<StatusCode> {
    let proj: NewProject = req.body_json().await?;
    let id = req.param::<Uuid>("id")?;

    let res = sqlx::query(
        "UPDATE project
        SET name = $2,
            description = $3
        WHERE id = $1",
    )
    .bind(&id)
    .bind(&proj.name)
    .bind(&proj.description)
    .execute(&req.get_pool())
    .await;

    match pg_error(res)? {
        Ok(_done) => {
            info!("updated project {} -> {}", proj.name, id);
            Ok(StatusCode::Created)
        }
        Err(err) => {
            warn!("error creating project: {}", err);
            if &err.code()[..2] == PG_INTEGRITY_ERROR {
                Ok(StatusCode::Conflict)
            } else {
                Ok(StatusCode::InternalServerError)
            }
        }
    }
}

#[derive(Deserialize)]
struct QueryProject {
    pub name: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
struct Project {
    pub id: Uuid,
    pub name: String,
    pub description: String,
}

pub async fn list(req: Request<State>) -> tide::Result<Response> {
    let projs = sqlx::query_as::<_, Project>(
        "SELECT id, name, description
        FROM project",
    )
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Response::ok().json(projs)?)
}

pub async fn get_by_name(req: Request<State>) -> impl Responder {
    let q = req.query::<QueryProject>()?;

    if let Some(name) = q.name {
        let row = sqlx::query_as::<_, Project>(
            "SELECT id, name, description
            FROM project
            WHERE name = $1",
        )
        .bind(&name)
        .fetch_optional(&req.get_pool())
        .await?;

        Ok(match row {
            None => Response::status(StatusCode::NotFound),
            Some(proj) => Response::ok().json(proj)?,
        })
    } else {
        list(req).await
    }
}

#[derive(Serialize, sqlx::FromRow)]
struct ProjectExtra {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub num_jobs: i64,
    pub active_tasks: i64,
    pub failed_tasks_last_hour: i64,
    pub succeeded_tasks_last_hour: i64,
}

pub async fn get_by_id(req: Request<State>) -> tide::Result<Response> {
    let id_str = req.param::<String>("id")?;
    let id = Uuid::parse_str(&id_str)?;

    let row = sqlx::query_as::<_, ProjectExtra>(
        "SELECT
            id,
            name,
            description,
            (
                SELECT count(1)
                FROM job j
                WHERE j.project_id = $1
            ) AS num_jobs,
            (
                SELECT count(1)
                FROM job j
                JOIN task t ON t.job_id = j.id
                JOIN task_run tr ON tr.task_id = t.id
                WHERE j.project_id = $1
                AND tr.state = 'active'
            ) AS active_tasks,
            (
                SELECT count(1)
                FROM job j
                JOIN task t ON t.job_id = j.id
                JOIN task_run tr ON tr.task_id = t.id
                WHERE j.project_id = $1
                AND tr.state = 'failure'
                AND CURRENT_TIMESTAMP - finish_datetime < INTERVAL '1 hour'
            ) AS failed_tasks_last_hour,
            (
                SELECT count(1)
                FROM job j
                JOIN task t ON t.job_id = j.id
                JOIN task_run tr ON tr.task_id = t.id
                WHERE j.project_id = $1
                AND tr.state = 'success'
                AND CURRENT_TIMESTAMP - finish_datetime < INTERVAL '1 hour'
            ) AS succeeded_tasks_last_hour
        FROM project
        WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&req.get_pool())
    .await?;

    Ok(match row {
        None => Response::status(StatusCode::NotFound),
        Some(proj) => Response::ok().json(proj)?,
    })
}

pub async fn delete(req: Request<State>) -> tide::Result<StatusCode> {
    let id_str = req.param::<String>("id")?;
    let id = Uuid::parse_str(&id_str)?;

    let res = sqlx::query(
        "DELETE FROM project
        WHERE id = $1",
    )
    .bind(&id)
    .execute(&req.get_pool())
    .await;

    match pg_error(res)? {
        Ok(done) => {
            if done.rows_affected() == 1 {
                info!("deleted project {}", id);
                Ok(StatusCode::NoContent)
            } else {
                info!("no project with id {}", id);
                Ok(StatusCode::NotFound)
            }
        }
        Err(err) => {
            warn!("error deleting project: {}", err);
            Ok(StatusCode::InternalServerError)
        }
    }
}

#[derive(Serialize, sqlx::FromRow)]
struct ListJob {
    job_id: Uuid,
    name: String,
    description: String,
}

pub async fn list_jobs(req: Request<State>) -> tide::Result<impl Responder> {
    let id_str = req.param::<String>("id")?;
    let id = Uuid::parse_str(&id_str)?;

    let jobs = sqlx::query_as::<_, ListJob>(
        "SELECT
            id AS job_id,
            name,
            description
        FROM job
        WHERE project_id = $1",
    )
    .bind(&id)
    .fetch_all(&req.get_pool())
    .await?;

    // TODO - check for project_id not found

    Ok(Json(jobs))
}
