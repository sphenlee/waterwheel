use crate::server::api::{State, request_ext::RequestExt};
use highnoon::{Json, Request, Responder, StatusCode};
use kv_log_macro::info;
use uuid::Uuid;

use super::{StashName, StashData, get_jwt_subject};

pub async fn create(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let data = req.body_bytes().await?;

    let proj_id = req.param("id")?.parse::<Uuid>()?;
    let key = req.param("key")?;

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

    info!("created project stash item", {
        project_id: proj_id.to_string(),
        key: key
    });

    Ok(StatusCode::CREATED)
}

pub async fn list(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let proj_id = req.param("id")?.parse::<Uuid>()?;

    let rows = sqlx::query_as::<_, StashName>(
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

    info!("task requested project stash", {
        project_id: proj_id.to_string(),
        task_id: task_id.to_string(),
        key: key,
    });

    let row = sqlx::query_as::<_, StashData>(
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

    Ok(row)
}

pub async fn delete(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let proj_id = req.param("id")?.parse::<Uuid>()?;
    let key = req.param("key")?;

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

    info!("deleted project stash item", {
        project_id: proj_id.to_string(),
        key: key
    });

    Ok(StatusCode::NO_CONTENT)
}
