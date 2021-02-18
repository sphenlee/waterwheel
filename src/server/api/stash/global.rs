use crate::server::api::{State, request_ext::RequestExt};
use highnoon::{Json, Request, Responder, StatusCode};
use kv_log_macro::info;

use super::{StashName, StashData, get_jwt_subject};

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

    info!("created global stash item", { key: key });

    Ok(StatusCode::CREATED)
}

pub async fn list(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let rows = sqlx::query_as::<_, StashName>(
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

    info!("task requested global stash", {
        task_id: subject,
        key: key,
    });

    let row = sqlx::query_as::<_, StashData>(
        "SELECT data
        FROM global_stash
        WHERE name = $1",
    )
        .bind(&key)
        .fetch_optional(&db)
        .await?;

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

    info!("deleted global stash item", { key: key });

    Ok(StatusCode::NO_CONTENT)
}
