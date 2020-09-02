use crate::server::api::util::RequestExt;
use crate::server::api::State;
use chrono::{DateTime, Utc};
use hightide::{Json, Responder};
use serde::{Deserialize, Serialize};
use sqlx::Done;
use tide::Request;
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

pub async fn get_tokens(req: Request<State>) -> tide::Result<impl Responder> {
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

    Ok(Json(tokens))
}

pub async fn get_tokens_trigger_datetime(req: Request<State>) -> tide::Result<impl Responder> {
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
        WHERE t.job_id = $1
        AND k.trigger_datetime = $2",
    )
    .bind(&job_id)
    .bind(&trigger_datetime)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(tokens))
}

#[derive(Serialize)]
struct ClearTokens {
    tokens_cleared: u64,
}

pub async fn clear_tokens_trigger_datetime(req: Request<State>) -> tide::Result<impl Responder> {
    let job_id = req.param::<Uuid>("id")?;
    let trigger_datetime = req.param::<DateTime<Utc>>("trigger_datetime")?;

    let done = sqlx::query(
        "UPDATE token k
         SET count = 0,
             state = 'waiting'
        FROM task t
        WHERE k.task_id = t.id
        AND t.job_id = $1
        AND k.trigger_datetime = $2",
    )
    .bind(&job_id)
    .bind(&trigger_datetime)
    .execute(&req.get_pool())
    .await?;

    let body = ClearTokens {
        tokens_cleared: done.rows_affected(),
    };

    Ok(Json(body))
}
