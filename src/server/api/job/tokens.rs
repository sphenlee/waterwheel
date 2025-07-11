use crate::{
    messages::{ProcessToken, Token, TokenState},
    server::api::{State, auth, request_ext::RequestExt, updates},
};
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder};
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::BTreeMap};
use uuid::Uuid;

#[derive(Deserialize)]
struct QueryToken {
    state: Option<String>,
    before: Option<DateTime<Utc>>,
    limit: Option<i32>,
}

#[derive(Serialize, sqlx::FromRow)]
struct GetToken {
    task_id: Uuid,
    task_name: String,
    trigger_datetime: DateTime<Utc>,
    state: String,
}

async fn get_tokens_common(req: Request<State>) -> highnoon::Result<Vec<GetToken>> {
    let job_id = req.param("id")?.parse::<Uuid>()?;
    let q = req.query::<QueryToken>()?;

    auth::get().job(job_id, None).check(&req).await?;

    let maybe_states: Option<Vec<_>> = q.state.as_ref().map(|s| s.split(',').collect());

    if let Some(states) = &maybe_states {
        for state in states {
            let _ = state
                .parse::<TokenState>()
                .map_err(|err| highnoon::Error::bad_request(err.0))?;
        }
    }

    let tokens: Vec<GetToken> = sqlx::query_as(
        "WITH these_tokens AS (
            SELECT
                t.id AS task_id,
                t.name AS task_name,
                k.trigger_datetime AS trigger_datetime,
                k.state AS state
            FROM task t
            JOIN token k ON k.task_id = t.id
            WHERE t.job_id = $1
            AND ($4 IS NULL OR state = ANY($4))
        ),
        these_datetimes AS (
            SELECT DISTINCT
                trigger_datetime
            FROM these_tokens
            WHERE ($2 IS NULL OR trigger_datetime < $2)
            ORDER BY trigger_datetime DESC
            LIMIT $3
        )
        SELECT
            task_id,
            task_name,
            tt.trigger_datetime AS trigger_datetime,
            state
        FROM these_tokens tt
        JOIN these_datetimes td ON td.trigger_datetime = tt.trigger_datetime
        ORDER BY trigger_datetime DESC
        ",
    )
    .bind(job_id)
    .bind(q.before)
    .bind(q.limit.unwrap_or(200))
    .bind(maybe_states)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(tokens)
}

pub async fn get_tokens(req: Request<State>) -> highnoon::Result<impl Responder> {
    let tokens = get_tokens_common(req).await?;
    Ok(Json(tokens))
}

#[derive(Serialize)]
struct TokenOverviewState {
    task_name: String,
    task_id: Uuid,
    state: String,
}

#[derive(Serialize)]
struct TokenOverviewRow {
    trigger_datetime: DateTime<Utc>,
    task_states: BTreeMap<String, TokenOverviewState>,
}

#[derive(Serialize)]
struct GetTokensOverview {
    tokens: Vec<TokenOverviewRow>,
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

    let mut tokens_by_time = BTreeMap::<DateTime<Utc>, BTreeMap<String, TokenOverviewState>>::new();

    for token in &tokens {
        tokens_by_time
            .entry(token.trigger_datetime)
            .or_default()
            .insert(
                token.task_name.clone(),
                TokenOverviewState {
                    task_name: token.task_name.clone(),
                    task_id: token.task_id,
                    state: token.state.clone(),
                },
            );
    }

    let mut tokens_by_time = tokens_by_time
        .into_iter()
        .map(|(k, v)| TokenOverviewRow {
            trigger_datetime: k,
            task_states: v,
        })
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

    auth::get().job(job_id, None).check(&req).await?;

    let tokens: Vec<GetToken> = sqlx::query_as(
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
    .bind(job_id)
    .bind(trigger_datetime)
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

    auth::delete().job(job_id, None).check(&req).await?;

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
    .bind(job_id)
    .bind(trigger_datetime)
    .fetch_all(&req.get_pool())
    .await?;

    for &(id,) in &task_ids {
        let token = Token {
            task_id: id,
            trigger_datetime,
        };
        updates::send_token_update(req.get_channel(), ProcessToken::Clear(token)).await?;
    }

    let body = ClearTokens {
        tokens_cleared: task_ids.len() as u64,
    };

    Ok(Json(body))
}
