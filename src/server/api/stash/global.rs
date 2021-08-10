use crate::server::api::{request_ext::RequestExt, State};
use highnoon::{Json, Request, Responder, StatusCode};
use tracing::info;

use super::{get_jwt_subject, StashData, StashName};
use cadence::CountedExt;

pub async fn create(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let data = req.body_bytes().await?;
    let key = req.param("key")?;

    let db = req.get_pool();

    sqlx::query(
        "INSERT INTO global_stash(name, data)
        VALUES ($1, $2)
        ON CONFLICT (name)
        DO UPDATE
        SET data = $2",
    )
    .bind(&key)
    .bind(&data)
    .execute(&db)
    .await?;

    info!(key, "created global stash item");

    Ok(StatusCode::CREATED)
}

pub async fn list(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let rows: Vec<StashName> = sqlx::query_as(
        "SELECT name
        FROM global_stash",
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}

pub async fn get(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let subject = get_jwt_subject(&req)?;
    let key = req.param("key")?;

    info!(task_id=?subject, key, "task requested global stash");

    let row: Option<StashData> = sqlx::query_as(
        "SELECT data
        FROM global_stash
        WHERE name = $1",
    )
    .bind(&key)
    .fetch_optional(&db)
    .await?;

    req.state().statsd.incr_with_tags("stash.get")
        .with_tag_value("global")
        .send();

    Ok(row)
}

pub async fn delete(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let key = req.param("key")?;

    let _done = sqlx::query(
        "DELETE
        FROM global_stash
        WHERE name = $1",
    )
    .bind(&key)
    .execute(&db)
    .await?;

    info!(key, "deleted global stash item");

    Ok(StatusCode::NO_CONTENT)
}
