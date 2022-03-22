use crate::server::api::{auth, request_ext::RequestExt, State};
use highnoon::{Json, Request, Responder, StatusCode};
use tracing::info;
use uuid::Uuid;

use super::{get_jwt_subject, StashData, StashName};
use cadence::CountedExt;

pub async fn create(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let data = req.body_bytes().await?;

    let proj_id = req.param("id")?.parse::<Uuid>()?;
    let key = req.param("key")?;

    auth::update()
        .project(proj_id)
        .kind("stash")
        .check(&req)
        .await?;

    let db = req.get_pool();

    sqlx::query(
        "INSERT INTO project_stash(project_id, name, data)
        VALUES ($1, $2, $3)
        ON CONFLICT (project_id, name)
        DO UPDATE
        SET data = $3",
    )
    .bind(&proj_id)
    .bind(&key)
    .bind(&data)
    .execute(&db)
    .await?;

    info!(project_id=?proj_id, key, "created project stash item");

    Ok(StatusCode::CREATED)
}

pub async fn list(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let proj_id = req.param("id")?.parse::<Uuid>()?;

    auth::list()
        .project(proj_id)
        .kind("stash")
        .check(&req)
        .await?;

    let rows: Vec<StashName> = sqlx::query_as(
        "SELECT name
        FROM project_stash
        WHERE project_id = $1",
    )
    .bind(&proj_id)
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}

pub async fn get(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let proj_id = req.param("id")?.parse::<Uuid>()?;
    let task_id = get_jwt_subject(&req)?.parse::<Uuid>()?;
    let key = req.param("key")?;

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
    .bind(&proj_id)
    .bind(&task_id)
    .bind(&key)
    .fetch_optional(&db)
    .await?;

    req.get_statsd()
        .incr_with_tags("stash.get")
        .with_tag_value("project")
        .with_tag("proj_id", &proj_id.to_string())
        .send();

    Ok(row)
}

pub async fn delete(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let proj_id = req.param("id")?.parse::<Uuid>()?;
    let key = req.param("key")?;

    auth::delete()
        .project(proj_id)
        .kind("stash")
        .check(&req)
        .await?;

    let _done = sqlx::query(
        "DELETE
        FROM project_stash
        WHERE project_id = $1
        AND name = $2",
    )
    .bind(&proj_id)
    .bind(&key)
    .execute(&db)
    .await?;

    info!(?proj_id, ?key, "deleted project stash item");

    Ok(StatusCode::NO_CONTENT)
}
