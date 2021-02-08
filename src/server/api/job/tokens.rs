use crate::messages::Token;
use crate::postoffice;
use crate::server::api::request_ext::RequestExt;
use crate::server::api::State;
use crate::server::tokens::ProcessToken;
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder};
use postage::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Deserialize)]
struct QueryToken {
    state: Option<String>,
    //from: Option<DateTime<Utc>>
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

async fn get_tokens_common(req: Request<State>) -> highnoon::Result<Vec<GetToken>> {
    let job_id = req.param("id")?.parse::<Uuid>()?;
    let q = req.query::<QueryToken>()?;

    let states: Option<Vec<_>> = q
        .state
        .map(|s| s.split(',').map(|s| s.to_owned()).collect());
    //let from = q.from;

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
        AND ($2 IS NULL OR k.state = ANY($2))
        --AND ($3 IS NULL OR k.trigger_datetime > $3)
        ORDER BY k.trigger_datetime DESC
        LIMIT 100",
    )
    .bind(job_id)
    .bind(&states)
    //.bind(&from)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(tokens)
}

pub async fn get_tokens(req: Request<State>) -> highnoon::Result<impl Responder> {
    let tokens = get_tokens_common(req).await?;
    Ok(Json(tokens))
}

#[derive(Serialize)]
struct TokenState {
    task_name: String,
    task_id: Uuid,
    state: String,
}

#[derive(Serialize)]
struct TokensRow {
    trigger_datetime: DateTime<Utc>,
    task_states: BTreeMap<String, TokenState>,
}

#[derive(Serialize)]
struct GetTokensOverview {
    tokens: Vec<TokensRow>,
    tasks: Vec<String>,
}

pub async fn get_tokens_overview(req: Request<State>) -> highnoon::Result<impl Responder> {
    let tokens = get_tokens_common(req).await?;

    let mut tasks = tokens
        .iter()
        .map(|t| t.task_name.clone())
        .collect::<Vec<_>>();

    tasks.sort();
    tasks.dedup();

    let mut tokens_by_time = BTreeMap::<DateTime<Utc>, BTreeMap<String, TokenState>>::new();

    for token in &tokens {
        tokens_by_time
            .entry(token.trigger_datetime.clone())
            .or_default()
            .insert(
                token.task_name.clone(),
                TokenState {
                    task_name: token.task_name.clone(),
                    task_id: token.task_id.clone(),
                    state: token.state.clone(),
                },
            );
    }

    let mut tokens_by_time = tokens_by_time
        .into_iter()
        .map(|(k, v)| TokensRow {
            trigger_datetime: k,
            task_states: v,
        })
        //.take(50) // TODO - change this value
        .collect::<Vec<_>>();

    tokens_by_time.sort_by_key(|item| Reverse(item.trigger_datetime));

    Ok(Json(GetTokensOverview {
        tokens: tokens_by_time,
        tasks,
    }))
}

pub async fn get_tokens_trigger_datetime(req: Request<State>) -> highnoon::Result<impl Responder> {
    let job_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;

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
        AND k.trigger_datetime = $2
        ORDER BY t.name",
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

pub async fn clear_tokens_trigger_datetime(
    req: Request<State>,
) -> highnoon::Result<impl Responder> {
    let job_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;

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

    let mut tokens_tx = postoffice::post_mail::<ProcessToken>().await?;
    for (id,) in &task_ids {
        let token = Token {
            task_id: id.clone(),
            trigger_datetime: trigger_datetime.clone(),
        };
        tokens_tx.send(ProcessToken::Clear(token)).await?;
    }

    let body = ClearTokens {
        tokens_cleared: task_ids.len() as u64,
    };

    Ok(Json(body))
}
