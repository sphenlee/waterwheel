use super::request_ext::RequestExt;
use super::State;
use crate::util::{pg_error, is_pg_integrity_error};
use highnoon::{Json, Request, Responder, Response, StatusCode};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
struct NewProject {
    pub uuid: Option<Uuid>,
    pub name: String,
    pub description: String,
}

pub async fn create(mut req: Request<State>) -> highnoon::Result<Response> {
    let proj: NewProject = req.body_json().await?;

    let id = proj.uuid.unwrap_or_else(uuid::Uuid::new_v4);

    let res = sqlx::query(
        "INSERT INTO project(id, name, description)
        VALUES($1, $2, $3)
        ON CONFLICT(id)
        DO UPDATE
        SET name = $2,
            description = $3",
    )
    .bind(&id)
    .bind(&proj.name)
    .bind(&proj.description)
    .execute(&req.get_pool())
    .await;

    match pg_error(res)? {
        Ok(_done) => {
            info!("updated project {} -> {}", id, proj.name);
            let proj = Project {
                id,
                name: proj.name,
                description: proj.description,
            };
            Ok(Response::status(StatusCode::CREATED).json(proj)?)
        }
        Err(err) => {
            warn!("error updating project: {}", err);
            if is_pg_integrity_error(&err) {
                Ok(Response::status(StatusCode::CONFLICT)
                    .body("a project with this name already exists"))
            } else {
                Ok(Response::status(StatusCode::INTERNAL_SERVER_ERROR))
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

pub async fn list(req: Request<State>) -> highnoon::Result<Response> {
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
            None => Response::status(StatusCode::NOT_FOUND),
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

pub async fn get_by_id(req: Request<State>) -> highnoon::Result<Response> {
    let id_str = req.param("id")?;
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
        None => Response::status(StatusCode::NOT_FOUND),
        Some(proj) => Response::ok().json(proj)?,
    })
}

pub async fn delete(req: Request<State>) -> highnoon::Result<StatusCode> {
    let id_str = req.param("id")?;
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
                Ok(StatusCode::NO_CONTENT)
            } else {
                info!("no project with id {}", id);
                Ok(StatusCode::NOT_FOUND)
            }
        }
        Err(err) => {
            warn!("error deleting project: {}", err);
            Ok(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[derive(Serialize, sqlx::FromRow)]
struct ListJob {
    job_id: Uuid,
    name: String,
    description: String,
}

pub async fn list_jobs(req: Request<State>) -> highnoon::Result<impl Responder> {
    let id_str = req.param("id")?;
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
