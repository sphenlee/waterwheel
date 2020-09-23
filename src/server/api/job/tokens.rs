use crate::server::api::util::RequestExt;
use crate::server::api::State;
use crate::postoffice;
use chrono::{DateTime, Utc};
use hightide::{Json, Responder};
use serde::{Deserialize, Serialize};
use tide::Request;
use uuid::Uuid;
use crate::server::tokens::ProcessToken;
use crate::messages::Token;

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

    let task_ids: Vec<(Uuid,)> = sqlx::query_as(
        "UPDATE token k
        SET count = 0,
            state = 'waiting'
        FROM task t
        WHERE k.task_id = t.id
        AND t.job_id = $1
        AND k.trigger_datetime = $2
        RETURNING k.task_id",
    )
    .bind(&job_id)
    .bind(&trigger_datetime)
    .fetch_all(&req.get_pool())
    .await?;

    let tokens_tx = postoffice::post_mail::<ProcessToken>().await?;
    for (id,) in &task_ids {
        let token = Token {
            task_id: id.clone(),
            trigger_datetime: trigger_datetime.clone()
        };
        tokens_tx.send(ProcessToken::Clear(token)).await;
    }

    let body = ClearTokens {
        tokens_cleared: task_ids.len() as u64,
    };

    Ok(Json(body))
}
