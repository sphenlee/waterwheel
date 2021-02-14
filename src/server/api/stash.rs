use super::request_ext::RequestExt;
use super::State;
use highnoon::{Json, Request, Responder, StatusCode};
use kv_log_macro::{info};
use uuid::Uuid;

#[derive(sqlx::FromRow, serde::Serialize)]
struct StashName(String);

#[derive(sqlx::FromRow)]
struct StashData(Vec<u8>);

impl Responder for StashData {
    fn into_response(self) -> highnoon::Result<highnoon::Response> {
        self.0.into_response()
    }
}

pub async fn create_global_stash(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let data = req.body_bytes().await?;
    let key = req.param("key")?;

    let db = req.get_pool();

    sqlx::query(
        "INSERT INTO global_stash(name, data)
        VALUES ($1, $2)
        ON CONFLICT (name)
        DO UPDATE
        SET data = $2")
        .bind(&key)
        .bind(&data)
        .execute(&db)
        .await?;

    info!("created global stash item", {key: key});

    Ok(StatusCode::CREATED)
}

pub async fn list_global_stash(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let rows = sqlx::query_as::<_, StashName>(
        "SELECT name
        FROM global_stash")
        .fetch_all(&db)
        .await?
        ;

    Ok(Json(rows))
}

pub async fn get_global_stash(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let key = req.param("key")?;

    let row = sqlx::query_as::<_, StashData>(
        "SELECT data
        FROM global_stash
        WHERE name = $1")
        .bind(&key)
        .fetch_optional(&db)
        .await?;

    Ok(row)
}

pub async fn delete_global_stash(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let key = req.param("key")?;

    let done = sqlx::query(
        "DELETE
        FROM global_stash
        WHERE name = $1")
        .bind(&key)
        .execute(&db)
        .await?;


    info!("deleted global stash item", {key: key});

    Ok(StatusCode::NO_CONTENT)
}


pub async fn create_project_stash(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let data = req.body_bytes().await?;

    let proj_id = req.param("id")?.parse::<Uuid>()?;
    let key = req.param("key")?;

    let db = req.get_pool();

    sqlx::query(
        "INSERT INTO project_stash(project_id, name, data)
        VALUES ($1, $2, $3)
        ON CONFLICT (project_id, name)
        DO UPDATE
        SET data = $3")
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

pub async fn list_project_stash(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let proj_id = req.param("id")?.parse::<Uuid>()?;

    let rows = sqlx::query_as::<_, StashName>(
        "SELECT name
        FROM project_stash
        WHERE project_id = $1")
        .bind(&proj_id)
        .fetch_all(&db)
        .await?
        ;

    Ok(Json(rows))
}

pub async fn get_project_stash(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let proj_id = req.param("id")?.parse::<Uuid>()?;
    let key = req.param("key")?;

    let row = sqlx::query_as::<_, StashData>(
        "SELECT data
        FROM project_stash
        WHERE project_id = $1
        AND name = $2")
        .bind(&proj_id)
        .bind(&key)
        .fetch_optional(&db)
        .await?;

    Ok(row)
}

pub async fn delete_project_stash(req: Request<State>) -> highnoon::Result<impl Responder> {
    let db = req.get_pool();

    let proj_id = req.param("id")?.parse::<Uuid>()?;
    let key = req.param("key")?;

    let done = sqlx::query(
        "DELETE
        FROM project_stash
        WHERE project_id = $1
        AND name = $2")
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
