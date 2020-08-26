use crate::server::api::util::RequestExt;
use crate::server::api::State;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tide::{Body, Request, Response, StatusCode};
use uuid::Uuid;

#[derive(Deserialize)]
struct QueryToken {
    state: String,
}

#[derive(Serialize, sqlx::FromRow)]
struct GetToken {
    task_id: Uuid,
    task_name: String,
    threshold: i32,
    count: i32,
    trigger_datetime: DateTime<Utc>,
    state: String,
}

pub async fn get_tokens(req: Request<State>) -> tide::Result {
    let job_id = req.param::<Uuid>("id")?;
    let q = req.query::<QueryToken>()?;

    let states: Vec<_> = q.state.split(',').map(|s| s.to_owned()).collect();

    let tokens = sqlx::query_as::<_, GetToken>(
        "SELECT
            t.id AS task_id,
            t.name AS task_name,
            t.threshold AS threshold,
            k.count AS count,
            k.trigger_datetime AS trigger_datetime,
            k.state AS state
        FROM task t
        JOIN token k ON k.task_id = t.id
        AND t.job_id = $1
        AND k.state = ANY($2)
        ORDER BY k.trigger_datetime DESC",
    )
    .bind(&job_id)
    .bind(&states)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Response::builder(StatusCode::Ok)
        .body(Body::from_json(&tokens)?)
        .build())
}

pub async fn get_token_trigger_datetime(req: Request<State>) -> tide::Result {
    let job_id = req.param::<Uuid>("id")?;
    let trigger_datetime = req.param::<DateTime<Utc>>("trigger_datetime")?;

    let tokens = sqlx::query_as::<_, GetToken>(
        "SELECT
            t.id AS task_id,
            t.name AS task_name,
            t.threshold AS threshold,
            k.count AS count,
            k.trigger_datetime AS trigger_datetime,
            k.state AS state
        FROM task t
        JOIN token k ON k.task_id = t.id
        AND t.job_id = $1
        AND k.trigger_datetime = $2",
    )
    .bind(&job_id)
    .bind(&trigger_datetime)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Response::builder(StatusCode::Ok)
        .body(Body::from_json(&tokens)?)
        .build())
}
