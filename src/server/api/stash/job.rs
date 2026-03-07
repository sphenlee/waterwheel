use crate::server::api::{App, AppResult, auth, stash::JwtSubject};
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
use chrono::{DateTime, Utc};

#[axum::debug_handler]
pub async fn create(
    State(app): State<App>,
    Path((job_id, trigger_datetime, key)): Path<(Uuid, DateTime<Utc>, String)>,
    JwtSubject(subject): JwtSubject,
    data: Bytes,
) -> AppResult<Response> {
    let task_id = subject.parse::<Uuid>()?;

    // don't check authz here - job stash are expected to be created by tasks
    // and so we want to check permissions using the Stash JWT

    let db = app.get_pool();

    sqlx::query(
        "INSERT INTO job_stash(job_id, trigger_datetime, name, data)
        SELECT $1, $2, $3, $4
        WHERE (
            SELECT TRUE
            FROM task
            WHERE id = $5
            AND job_id = $1
        )
        ON CONFLICT (job_id, trigger_datetime, name)
        DO UPDATE
        SET data = $4",
    )
    .bind(job_id)
    .bind(trigger_datetime)
    .bind(&key)
    .bind(data.as_ref())
    .bind(task_id)
    .execute(&db)
    .await?;

    info!(?job_id, trigger_datetime=?trigger_datetime.to_rfc3339(), %key, "created job stash item");

    Ok(StatusCode::CREATED.into_response())
}

#[axum::debug_handler]
pub async fn list(
    State(app): State<App>,
    Path((job_id, trigger_datetime)): Path<(Uuid, DateTime<Utc>)>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let db = app.get_pool();

    auth::list(&app)
        .job(job_id, None)
        .kind("stash")
        .check(&headers)
        .await?;

    let rows: Vec<StashName> = sqlx::query_as(
        "SELECT name
        FROM job_stash
        WHERE job_id = $1
        AND trigger_datetime = $2",
    )
    .bind(job_id)
    .bind(trigger_datetime)
    .fetch_all(&db)
    .await?;

    Ok(Json(rows).into_response())
}

#[axum::debug_handler]
pub async fn get(
    State(app): State<App>,
    JwtSubject(subject): JwtSubject,
    Path((job_id, trigger_datetime, key)): Path<(Uuid, DateTime<Utc>, String)>,
) -> AppResult<Response> {
    let db = app.get_pool();

    let task_id = subject.parse::<Uuid>()?;

    info!(?job_id,
        trigger_datetime=?trigger_datetime.to_rfc3339(),
        ?task_id,
        %key,
        "task requested job stash");

    let row: Option<StashData> = sqlx::query_as(
        "SELECT js.data
        FROM job_stash js
        WHERE js.job_id = $1
        AND js.trigger_datetime = $2
        AND (SELECT TRUE
             FROM task t
             WHERE t.id = $3
             AND t.job_id = $1)
        AND js.name = $4",
    )
    .bind(job_id)
    .bind(trigger_datetime)
    .bind(task_id)
    .bind(&key)
    .fetch_optional(&db)
    .await?;

    app.get_statsd()
        .incr_with_tags("stash.get")
        .with_tag_value("job")
        .with_tag("job_id", &job_id.to_string())
        .send();

    match row {
        Some(data) => Ok(data.into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[axum::debug_handler]
pub async fn delete(
    State(app): State<App>,
    Path((job_id, trigger_datetime, key)): Path<(Uuid, DateTime<Utc>, String)>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let db = app.get_pool();

    auth::delete(&app)
        .job(job_id, None)
        .kind("stash")
        .check(&headers)
        .await?;

    let _done = sqlx::query(
        "DELETE
        FROM job_stash
        WHERE job_id = $1
        AND trigger_datetime = $2
        AND name = $3",
    )
    .bind(job_id)
    .bind(trigger_datetime)
    .bind(&key)
    .execute(&db)
    .await?;

    info!(?job_id, trigger_datetime=?trigger_datetime.to_rfc3339(), %key, "deleted job stash item");

    Ok(StatusCode::NO_CONTENT.into_response())
}
