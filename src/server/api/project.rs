use super::util::RequestExt;
use super::{pg_error, State, PG_INTEGRITY_ERROR};
use log::{info, warn};
use serde::{Deserialize, Serialize};
use sqlx::Done;
use tide::{Body, Request, Response, StatusCode};
use uuid::Uuid;

#[derive(Deserialize)]
struct NewProject {
    pub name: String,
}

pub async fn create(mut req: Request<State>) -> tide::Result<Response> {
    let proj: NewProject = req.body_json().await?;

    let id = uuid::Uuid::new_v4();

    let res = sqlx::query(
        "INSERT INTO project(id, name)
        VALUES($1, $2)",
    )
    .bind(&id)
    .bind(&proj.name)
    .execute(&req.get_pool())
    .await;

    match pg_error(res)? {
        Ok(_done) => {
            info!("created project {} -> {}", proj.name, id);
            let body = Body::from_json(&Project {
                id,
                name: proj.name,
            })?;
            Ok(Response::builder(StatusCode::Created).body(body).build())
        }
        Err(err) => {
            warn!("error creating project: {}", err);
            if &err.code()[..2] == PG_INTEGRITY_ERROR {
                Ok(Response::from(StatusCode::Conflict))
            } else {
                Ok(Response::from(StatusCode::InternalServerError))
            }
        }
    }
}

#[derive(Deserialize)]
struct QueryProject {
    pub name: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct Project {
    pub id: Uuid,
    pub name: String,
}

pub async fn get_by_name(req: Request<State>) -> tide::Result<Response> {
    let q = req.query::<QueryProject>()?;

    let row = sqlx::query_as::<_, Project>(
        "SELECT id, name
        FROM project
        WHERE name = $1",
    )
    .bind(&q.name)
    .fetch_optional(&req.get_pool())
    .await?;

    Ok(match row {
        None => Response::new(StatusCode::NotFound),
        Some(proj) => Response::builder(StatusCode::Ok)
            .body(Body::from_json(&proj)?)
            .build(),
    })
}

pub async fn get_by_id(req: Request<State>) -> tide::Result<Response> {
    let id_str = req.param::<String>("id")?;
    let id = Uuid::parse_str(&id_str)?;

    let row = sqlx::query_as::<_, Project>(
        "SELECT id, name
        FROM project
        WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&req.get_pool())
    .await?;

    Ok(match row {
        None => Response::new(StatusCode::NotFound),
        Some(proj) => Response::builder(StatusCode::Ok)
            .body(Body::from_json(&proj)?)
            .build(),
    })
}

pub async fn delete(req: Request<State>) -> tide::Result<StatusCode> {
    let id_str = req.param::<String>("id")?;
    let id = Uuid::parse_str(&id_str)?;

    let res = sqlx::query(
        "DELETE CASCADE FROM project
        WHERE id = $1",
    )
    .bind(&id)
    .execute(&req.get_pool())
    .await;

    match pg_error(res)? {
        Ok(done) => {
            if done.rows_affected() == 1 {
                info!("deleted project {}", id);
                Ok(StatusCode::NoContent)
            } else {
                info!("no project with id {}", id);
                Ok(StatusCode::NotFound)
            }
        }
        Err(err) => {
            warn!("error deleting project: {}", err);
            Ok(StatusCode::InternalServerError)
        }
    }
}
