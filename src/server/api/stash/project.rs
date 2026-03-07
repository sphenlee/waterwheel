use crate::server::api::{App, AppResult, auth, error::AppError, stash::JwtSubject};
use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use tracing::info;
use uuid::Uuid;

use super::{StashData, StashName};
use cadence::CountedExt;

pub async fn create(
    State(app): State<App>,
    Path((proj_id, key)): Path<(Uuid, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> AppResult<StatusCode> {
    auth::update(&app)
        .project(proj_id)
        .kind("stash")
        .check(&headers)
        .await?;

    let db = app.get_pool();

    sqlx::query(
        "INSERT INTO project_stash(project_id, name, data)
        VALUES ($1, $2, $3)
        ON CONFLICT (project_id, name)
        DO UPDATE
        SET data = $3",
    )
    .bind(proj_id)
    .bind(&key)
    .bind(body.as_ref())
    .execute(&db)
    .await?;

    info!(project_id=?proj_id, key, "created project stash item");

    Ok(StatusCode::CREATED)
}

pub async fn list(
    State(app): State<App>,
    Path(proj_id): Path<Uuid>,
    headers: HeaderMap,
) -> AppResult<Json<Vec<StashName>>> {
    let db = app.get_pool();

    auth::list(&app)
        .project(proj_id)
        .kind("stash")
        .check(&headers)
        .await?;

    let rows: Vec<StashName> = sqlx::query_as(
        "SELECT name
        FROM project_stash
        WHERE project_id = $1",
    )
    .bind(proj_id)
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}

pub async fn get(
    State(app): State<App>,
    JwtSubject(subject): JwtSubject,
    Path((proj_id, key)): Path<(Uuid, String)>,
) -> AppResult<Response> {
    let db = app.get_pool();

    let task_id = subject.parse::<Uuid>()?;

    info!(?proj_id, ?task_id, %key, "task requested project stash");

    let row: Option<StashData> = sqlx::query_as(
        "SELECT data
        FROM project_stash
        WHERE project_id = $1
        AND (SELECT TRUE
             FROM task t
             JOIN job j ON j.id = t.job_id
             WHERE t.id = $2
             AND j.project_id = $1
        )
        AND name = $3",
    )
    .bind(proj_id)
    .bind(task_id)
    .bind(key)
    .fetch_optional(&db)
    .await?;

    app.get_statsd()
        .incr_with_tags("stash.get")
        .with_tag_value("project")
        .with_tag("proj_id", &proj_id.to_string())
        .send();

    row.map(IntoResponse::into_response)
        .ok_or_else(|| AppError::http(StatusCode::NOT_FOUND))
}

pub async fn delete(
    State(app): State<App>,
    Path((proj_id, key)): Path<(Uuid, String)>,
    headers: HeaderMap,
) -> AppResult<StatusCode> {
    let db = app.get_pool();

    auth::delete(&app)
        .project(proj_id)
        .kind("stash")
        .check(&headers)
        .await?;

    let _done = sqlx::query(
        "DELETE
        FROM project_stash
        WHERE project_id = $1
        AND name = $2",
    )
    .bind(proj_id)
    .bind(&key)
    .execute(&db)
    .await?;

    info!(?proj_id, ?key, "deleted project stash item");

    Ok(StatusCode::NO_CONTENT)
}
