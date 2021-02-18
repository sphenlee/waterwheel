use crate::server::api::{State, request_ext::RequestExt};
use highnoon::{Json, Request, Responder, StatusCode};
use kv_log_macro::info;
use uuid::Uuid;

use super::{StashName, StashData, get_jwt_subject};
use chrono::{DateTime, Utc};

pub async fn create(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let data = req.body_bytes().await?;

    let job_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;
    let key = req.param("key")?;

    let db = req.get_pool();

    sqlx::query(
        "INSERT INTO job_stash(job_id, trigger_datetime, name, data)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (job_id, trigger_datetime, name)
        DO UPDATE
        SET data = $4",
    )
        .bind(&job_id)
        .bind(&trigger_datetime)
        .bind(&key)
        .bind(&data)
        .execute(&db)
        .await?;

    info!("created job stash item", {
        job_id: job_id.to_string(),
        trigger_datetime: trigger_datetime.to_rfc3339(),
        key: key
    });

    Ok(StatusCode::CREATED)
}

pub async fn list(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let job_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;

    let rows = sqlx::query_as::<_, StashName>(
        "SELECT name
        FROM job_stash
        WHERE job_id = $1
        AND trigger_datetime = $2",
    )
        .bind(&job_id)
        .bind(&trigger_datetime)
        .fetch_all(&db)
        .await?;

    Ok(Json(rows))
}

pub async fn get(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let job_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;
    let task_id = get_jwt_subject(&req)?.parse::<Uuid>()?;
    let key = req.param("key")?;

    info!("task requested job stash", {
        job_id: job_id.to_string(),
        trigger_datetime: trigger_datetime.to_rfc3339(),
        task_id: task_id.to_string(),
        key: key,
    });

    let row = sqlx::query_as::<_, StashData>(
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
        .bind(&job_id)
        .bind(&trigger_datetime)
        .bind(&task_id)
        .bind(&key)
        .fetch_optional(&db)
        .await?;

    Ok(row)
}

pub async fn delete(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let job_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;
    let key = req.param("key")?;

    let _done = sqlx::query(
        "DELETE
        FROM job_stash
        WHERE job_id = $1
        AND trigger_datetime = $2
        AND name = $3",
    )
        .bind(&job_id)
        .bind(&trigger_datetime)
        .bind(&key)
        .execute(&db)
        .await?;

    info!("deleted job stash item", {
        job_id: job_id.to_string(),
        trigger_datetime: trigger_datetime.to_rfc3339(),
        key: key
    });

    Ok(StatusCode::NO_CONTENT)
}
