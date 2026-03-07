use super::{App, AppResult, auth, config_cache};
use crate::{
    messages::ConfigUpdate,
    server::api::jwt,
    util::{is_pg_integrity_error, pg_error},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tracing::{info, warn};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct NewProject {
    pub uuid: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub config: Option<JsonValue>,
}

#[axum::debug_handler]
pub async fn create(
    State(app): State<App>,
    headers: HeaderMap,
    Json(proj): Json<NewProject>,
) -> AppResult<Response> {
    let id = proj.uuid.unwrap_or_else(uuid::Uuid::new_v4);

    auth::update(&app).project(id).check(&headers).await?;

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
    .execute(&app.get_pool())
    .await;

    match pg_error(res)? {
        Ok(_done) => {
            info!("updated project {} -> {}", id, proj.name);

            config_cache::send(app.get_channel(), ConfigUpdate::Project(id)).await?;

            let proj = NewProject {
                uuid: Some(id),
                ..proj
            };
            Ok((StatusCode::CREATED, Json(proj)).into_response())
        }
        Err(err) => {
            warn!("error updating project: {}", err);
            if is_pg_integrity_error(&err) {
                Ok((
                    StatusCode::CONFLICT,
                    "a project with this name already exists",
                )
                    .into_response())
            } else {
                Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response())
            }
        }
    }
}

#[derive(Deserialize)]
pub struct QueryProject {
    pub name: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ListProject {
    pub id: Uuid,
    pub name: String,
    pub description: String,
}

pub async fn list(State(app): State<App>, headers: HeaderMap) -> AppResult<Response> {
    auth::list(&app).project(None).check(&headers).await?;

    let projects: Vec<ListProject> = sqlx::query_as(
        "SELECT id, name, description
        FROM project
        ORDER BY name
        LIMIT 100",
    )
    .fetch_all(&app.get_pool())
    .await?;

    Ok(Json(projects).into_response())
}

pub async fn get_by_name(
    State(app): State<App>,
    Query(q): Query<QueryProject>,
    headers: HeaderMap,
) -> AppResult<Response> {
    if let Some(name) = q.name {
        let row: Option<ListProject> = sqlx::query_as(
            "SELECT id, name, description
            FROM project
            WHERE name = $1",
        )
        .bind(&name)
        .fetch_optional(&app.get_pool())
        .await?;

        match row {
            None => Ok(StatusCode::NOT_FOUND.into_response()),
            Some(proj) => {
                auth::get(&app).project(proj.id).check(&headers).await?;
                Ok(Json(proj).into_response())
            }
        }
    } else {
        list(State(app), headers).await
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

pub async fn get_by_id(
    State(app): State<App>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> AppResult<Response> {
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
    .fetch_optional(&app.get_pool())
    .await?;

    match row {
        None => Ok(StatusCode::NOT_FOUND.into_response()),
        Some(proj) => {
            auth::get(&app).project(proj.id).check(&headers).await?;
            Ok(Json(proj).into_response())
        }
    }
}

#[derive(sqlx::FromRow, Serialize)]
#[serde(transparent)]
struct ProjectConfig(JsonValue);

pub async fn get_config(
    State(app): State<App>,
    Path(id): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> AppResult<Response> {
    jwt::validate_config_jwt(&app, auth, id)?;

    let row: Option<ProjectConfig> = sqlx::query_as(
        "SELECT COALESCE(config, '{}'::jsonb) AS config
        FROM project
        WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&app.get_pool())
    .await?;

    if let Some(proj_conf) = row {
        Ok(Json(proj_conf).into_response())
    } else {
        Ok(StatusCode::NOT_FOUND.into_response())
    }
}

pub async fn delete(
    State(app): State<App>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    auth::delete(&app).project(id).check(&headers).await?;

    let res = sqlx::query(
        "DELETE FROM project
        WHERE id = $1",
    )
    .bind(id)
    .execute(&app.get_pool())
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
pub struct ListJobQuery {
    limit: Option<i32>,
    after: Option<String>,
    name: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ListJob {
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

pub async fn list_jobs(
    State(app): State<App>,
    Path(id): Path<Uuid>,
    Query(query): Query<ListJobQuery>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<ListJob>>> {
    auth::list(&app).project(id).check(&headers).await?;

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
    .fetch_all(&app.get_pool())
    .await?;

    Ok(Json(jobs))
}
