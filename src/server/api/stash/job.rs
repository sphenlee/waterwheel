use crate::server::api::{request_ext::RequestExt, State, auth};
use highnoon::{Json, Request, Responder, StatusCode};
use tracing::info;
use uuid::Uuid;

use super::{get_jwt_subject, StashData, StashName};
use cadence::CountedExt;
use chrono::{DateTime, Utc};

pub async fn create(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let data = req.body_bytes().await?;

    let job_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;
    let task_id = get_jwt_subject(&req)?.parse::<Uuid>()?;
    let key = req.param("key")?;

    // don't check authz here - job stash are expected to be created by tasks
    // and so we want to check permissions using the Stash JWT
    //auth::update().job(job_id).kind("stash").check(&req).await?;

    let db = req.get_pool();

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
    .bind(&job_id)
    .bind(&trigger_datetime)
    .bind(&key)
    .bind(&data)
    .bind(&task_id)
    .execute(&db)
    .await?;

    info!(?job_id, trigger_datetime=?trigger_datetime.to_rfc3339(), %key, "created job stash item");

    Ok(StatusCode::CREATED)
}

pub async fn list(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let job_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;

    auth::list().job(job_id, None).kind("stash").check(&req).await?;

    let rows: Vec<StashName> = sqlx::query_as(
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

    info!(?job_id,
        trigger_datetime=?trigger_datetime.to_rfc3339(),
        ?task_id,
        key,
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
    .bind(&job_id)
    .bind(&trigger_datetime)
    .bind(&task_id)
    .bind(&key)
    .fetch_optional(&db)
    .await?;

    req.state()
        .statsd
        .incr_with_tags("stash.get")
        .with_tag_value("job")
        .with_tag("job_id", &job_id.to_string())
        .send();

    Ok(row)
}

pub async fn delete(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let job_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;
    let key = req.param("key")?;

    auth::delete().job(job_id, None).kind("stash").check(&req).await?;

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

    info!(?job_id, trigger_datetime=?trigger_datetime.to_rfc3339(), key, "deleted job stash item");

    Ok(StatusCode::NO_CONTENT)
}
