use super::{State, auth, config_cache, request_ext::RequestExt};
use crate::{
    messages::ConfigUpdate,
    server::api::jwt,
    util::{is_pg_integrity_error, pg_error},
};
use highnoon::{Json, Request, Responder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct NewProject {
    pub uuid: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub config: Option<JsonValue>,
}

pub async fn create(mut req: Request<State>) -> highnoon::Result<Response> {
    let proj: NewProject = req.body_json().await?;

    let id = proj.uuid.unwrap_or_else(uuid::Uuid::new_v4);

    auth::update().project(id).check(&req).await?;

    let res = sqlx::query(
        "INSERT INTO project(id, name, description, config)
        VALUES($1, $2, $3, $4)
        ON CONFLICT(id)
        DO UPDATE
        SET name = $2,
            description = $3,
            config = COALESCE($4, project.config)",
    )
    .bind(id)
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
            (StatusCode::CREATED, Json(proj)).into_response()
        }
        Err(err) => {
            warn!("error updating project: {}", err);
            if is_pg_integrity_error(&err) {
                (
                    StatusCode::CONFLICT,
                    "a project with this name already exists",
                )
                    .into_response()
            } else {
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
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
    auth::list().project(None).check(&req).await?;

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

        match row {
            None => StatusCode::NOT_FOUND.into_response(),
            Some(proj) => {
                auth::get().project(proj.id).check(&req).await?;
                Json(proj).into_response()
            }
        }
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
    // TODO - harmonise these with the ListProject call
    pub running_tasks: i64,
    pub waiting_tasks: i64,
    pub failed_tasks_last_hour: i64,
    pub succeeded_tasks_last_hour: i64,
    pub error_tasks_last_hour: i64,
}

pub async fn get_by_id(req: Request<State>) -> highnoon::Result<Response> {
    let id_str = req.param("id")?;
    let id = Uuid::parse_str(id_str)?;

    let row: Option<ProjectExtra> = sqlx::query_as(
        "WITH these_tasks AS (
            SELECT
                t.id AS id,
                tr.state AS state
            FROM job j
            JOIN task t ON t.job_id = j.id
            JOIN task_run tr ON tr.task_id = t.id
            WHERE j.project_id = $1
            AND (finish_datetime IS NULL
                OR CURRENT_TIMESTAMP - finish_datetime < INTERVAL '1 hour')
        )
        SELECT
            id,
            name,
            description,
            (
                SELECT count(1)
                FROM job j
                WHERE j.project_id = $1
            ) AS num_jobs,
            (
                SELECT COUNT(1)
                FROM these_tasks t
                WHERE (t.state = 'running')
            ) AS running_tasks,
            (
                SELECT COUNT(1)
                FROM these_tasks t
                WHERE (t.state = 'waiting' OR t.state = 'active')
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
        FROM project
        WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&req.get_pool())
    .await?;

    match row {
        None => StatusCode::NOT_FOUND.into_response(),
        Some(proj) => {
            auth::get().project(proj.id).check(&req).await?;
            Json(proj).into_response()
        }
    }
}

#[derive(sqlx::FromRow, Serialize)]
#[serde(transparent)]
struct ProjectConfig(JsonValue);

pub async fn get_config(req: Request<State>) -> highnoon::Result<impl Responder> {
    let id_str = req.param("id")?;
    let id = Uuid::parse_str(id_str)?;

    jwt::validate_config_jwt(&req, id)?;

    let row: Option<ProjectConfig> = sqlx::query_as(
        "SELECT COALESCE(config, '{}'::jsonb) AS config
        FROM project
        WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&req.get_pool())
    .await?;

    if let Some(proj_conf) = row {
        Json(proj_conf).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn delete(req: Request<State>) -> highnoon::Result<StatusCode> {
    let id_str = req.param("id")?;
    let id = Uuid::parse_str(id_str)?;

    auth::delete().project(id).check(&req).await?;

    let res = sqlx::query(
        "DELETE FROM project
        WHERE id = $1",
    )
    .bind(id)
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

#[derive(Deserialize)]
struct ListJobQuery {
    limit: Option<i32>,
    after: Option<String>,
    name: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
struct ListJob {
    job_id: Uuid,
    name: String,
    description: String,
    paused: bool,
    success: i64,
    running: i64,
    failure: i64,
    waiting: i64,
    error: i64,
}

pub async fn list_jobs(req: Request<State>) -> highnoon::Result<impl Responder> {
    let id_str = req.param("id")?;
    let id = Uuid::parse_str(id_str)?;

    let query: ListJobQuery = req.query()?;

    auth::list().project(id).check(&req).await?;

    let jobs: Vec<ListJob> = sqlx::query_as(
        "WITH these_runs AS (
            SELECT
                t.job_id AS job_id,
                tr.state AS state
            FROM job j
            JOIN task t ON j.id = t.job_id
            LEFT OUTER JOIN task_run tr ON tr.task_id = t.id
            WHERE j.project_id = $1
            AND (
                tr.finish_datetime IS NULL
                OR CURRENT_TIMESTAMP - tr.finish_datetime < INTERVAL '1 hour'
                )
        ),
        job_stats AS (
            SELECT
                job_id,
                sum(CASE WHEN state = 'success' THEN 1 ELSE 0 END) AS success,
                sum(CASE WHEN state = 'running' THEN 1 ELSE 0 END) AS running,
                sum(CASE WHEN state = 'failure' THEN 1 ELSE 0 END) AS failure,
                sum(CASE
                        WHEN state = 'active' OR state = 'waiting' THEN 1
                        ELSE 0
                    END) AS waiting,
                sum(CASE WHEN state = 'error' THEN  1 ELSE 0 END) as error
            FROM these_runs
            GROUP BY job_id
        )
        SELECT
            id AS job_id,
            name,
            description,
            paused,
            coalesce(success, 0) AS success,
            coalesce(running, 0) AS running,
            coalesce(failure, 0) AS failure,
            coalesce(waiting, 0) AS waiting,
            coalesce(error,   0) AS error
        FROM job j
        LEFT OUTER JOIN job_stats js ON j.id = js.job_id
        WHERE project_id = $1
        AND ($2 IS NULL OR name > $2)
        AND ($3 IS NULL OR name = $3)
        ORDER BY name
        LIMIT $4",
    )
    .bind(id)
    .bind(query.after.as_ref())
    .bind(query.name.as_ref())
    .bind(query.limit.unwrap_or(50))
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(jobs))
}
