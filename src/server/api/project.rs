use super::request_ext::RequestExt;
use super::State;
use super::config_cache;
use crate::util::{is_pg_integrity_error, pg_error};
use highnoon::{Json, Request, Responder, Response, StatusCode};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue};
use uuid::Uuid;
use crate::messages::ConfigUpdate;

#[derive(Serialize, Deserialize)]
struct NewProject {
    pub uuid: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub config: Option<JsonValue>
}

pub async fn create(mut req: Request<State>) -> highnoon::Result<Response> {
    let proj: NewProject = req.body_json().await?;

    let id = proj.uuid.unwrap_or_else(uuid::Uuid::new_v4);

    let res = sqlx::query(
        "INSERT INTO project(id, name, description, config)
        VALUES($1, $2, $3, $4)
        ON CONFLICT(id)
        DO UPDATE
        SET name = $2,
            description = $3,
            config = COALESCE($4, project.config)",
    )
    .bind(&id)
    .bind(&proj.name)
    .bind(&proj.description)
    .bind(&proj.config)
    .execute(&req.get_pool())
    .await;

    match pg_error(res)? {
        Ok(_done) => {
            info!("updated project {} -> {}", id, proj.name);

            config_cache::send(req.get_channel(), ConfigUpdate::Project(id)).await?;

            let proj = NewProject {
                uuid: Some(id),
                ..proj
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
struct ListProject {
    pub id: Uuid,
    pub name: String,
    pub description: String,
}

pub async fn list(req: Request<State>) -> highnoon::Result<Response> {
    let projects: Vec<ListProject> = sqlx::query_as(
        "SELECT id, name, description
        FROM project
        ORDER BY name
        LIMIT 100",
    )
    .fetch_all(&req.get_pool())
    .await?;

    Json(projects).into_response()
}

pub async fn get_by_name(req: Request<State>) -> highnoon::Result<Response> {
    let q = req.query::<QueryProject>()?;

    if let Some(name) = q.name {
        let row: Option<ListProject> = sqlx::query_as(
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

    let row: Option<ProjectExtra> = sqlx::query_as(
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

#[derive(sqlx::FromRow, Serialize)]
#[serde(transparent)]
struct ProjectConfig(JsonValue);

pub async fn get_config(req: Request<State>) -> highnoon::Result<impl Responder> {
    let id_str = req.param("id")?;
    let id = Uuid::parse_str(&id_str)?;

    let row: Option<ProjectConfig> = sqlx::query_as(
        "SELECT COALESCE(config, '{}'::jsonb) AS config
        FROM project
        WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&req.get_pool())
    .await?;

    // two layers of option - outer layer is None if the project is not found
    // inner layer if the project has no config

    Ok(row.map(|config| Json(config.0)))
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

    let jobs: Vec<ListJob> = sqlx::query_as(
        "SELECT
            id AS job_id,
            name,
            description
        FROM job
        WHERE project_id = $1
        ORDER BY name
        LIMIT 200",
    )
    .bind(&id)
    .fetch_all(&req.get_pool())
    .await?;

    // TODO - check for project_id not found

    Ok(Json(jobs))
}
