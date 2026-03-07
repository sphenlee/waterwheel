use crate::server::api::{App, AppResult, auth, stash::JwtSubject};
use axum::{
    extract::{State, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum::body::Bytes;
use tracing::info;
use uuid::Uuid;

use super::{StashData, StashName};
use cadence::CountedExt;

#[axum::debug_handler]
pub async fn create(
    State(app): State<App>,
    headers: HeaderMap,
    Path(key): Path<String>,
    data: Bytes,
) -> AppResult<Response> {
    auth::update(&app).kind("stash").check(&headers).await?;

    let db = app.get_pool();

    sqlx::query(
        "INSERT INTO global_stash(name, data)
        VALUES ($1, $2)
        ON CONFLICT (name)
        DO UPDATE
        SET data = $2",
    )
    .bind(key.clone())
    .bind(data.as_ref())
    .execute(&db)
    .await?;

    info!(key, "created global stash item");

    Ok(StatusCode::CREATED.into_response())
}

#[axum::debug_handler]
pub async fn list(
    State(app): State<App>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let db = app.get_pool();

    auth::list(&app).kind("stash").check(&headers).await?;

    let rows: Vec<StashName> = sqlx::query_as(
        "SELECT name
        FROM global_stash",
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(rows).into_response())
}

#[axum::debug_handler]
pub async fn get(
    State(app): State<App>,
    JwtSubject(subject): JwtSubject,
    Path(key): Path<String>,
) -> AppResult<Response> {
    let db = app.get_pool();

    let task_id = subject.parse::<Uuid>()?;
    info!(?task_id, key, "task requested global stash");

    let row: Option<StashData> = sqlx::query_as(
        "SELECT data
        FROM global_stash
        WHERE name = $1",
    )
    .bind(&key)
    .fetch_optional(&db)
    .await?;

    app.get_statsd()
        .incr_with_tags("stash.get")
        .with_tag_value("global")
        .send();

    match row {
        Some(data) => Ok(data.into_response()),
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

#[axum::debug_handler]
pub async fn delete(
    State(app): State<App>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> AppResult<Response> {
    let db = app.get_pool();

    auth::delete(&app).kind("stash").check(&headers).await?;

    let _done = sqlx::query(
        "DELETE
        FROM global_stash
        WHERE name = $1",
    )
    .bind(&key)
    .execute(&db)
    .await?;

    info!(key, "deleted global stash item");

    Ok(StatusCode::NO_CONTENT.into_response())
}
