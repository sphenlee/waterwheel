use crate::{
    messages::{ProcessToken, Token, TokenState},
    server::api::{App, AppResult, auth, error::AppError, updates},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{cmp::Reverse, collections::BTreeMap};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct QueryToken {
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

// helper now takes extracted values rather than Request
async fn get_tokens_common(
    app: &App,
    headers: &HeaderMap,
    job_id: Uuid,
    q: &QueryToken,
) -> AppResult<Vec<GetToken>> {
    auth::get(app).job(job_id, None).check(headers).await?;

    let maybe_states: Option<Vec<_>> = q.state.as_ref().map(|s| s.split(',').collect());

    if let Some(states) = &maybe_states {
        for state in states {
            let _ = state
                .parse::<TokenState>()
                .map_err(|err| AppError::http((axum::http::StatusCode::BAD_REQUEST, err.0)))?;
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
    .fetch_all(&app.get_pool())
    .await?;

    Ok(tokens)
}

#[axum::debug_handler]
pub async fn get_tokens(
    State(app): State<App>,
    Path(job_id): Path<Uuid>,
    Query(q): Query<QueryToken>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let tokens = get_tokens_common(&app, &headers, job_id, &q).await?;
    Ok(Json(tokens).into_response())
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

#[axum::debug_handler]
pub async fn get_tokens_overview(
    State(app): State<App>,
    Path(job_id): Path<Uuid>,
    Query(q): Query<QueryToken>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let tokens = get_tokens_common(&app, &headers, job_id, &q).await?;

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
    })
    .into_response())
}

#[axum::debug_handler]
pub async fn get_tokens_trigger_datetime(
    State(app): State<App>,
    Path((job_id, trigger_datetime)): Path<(Uuid, DateTime<Utc>)>,
    headers: HeaderMap,
) -> AppResult<Response> {
    auth::get(&app).job(job_id, None).check(&headers).await?;

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
    .fetch_all(&app.get_pool())
    .await?;

    Ok(Json(tokens).into_response())
}

#[derive(Serialize)]
struct ClearTokens {
    tokens_cleared: u64,
}

#[axum::debug_handler]
pub async fn clear_tokens_trigger_datetime(
    State(app): State<App>,
    Path((job_id, trigger_datetime)): Path<(Uuid, DateTime<Utc>)>,
    headers: HeaderMap,
) -> AppResult<Response> {
    auth::delete(&app).job(job_id, None).check(&headers).await?;

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
    .fetch_all(&app.get_pool())
    .await?;

    for &(id,) in &task_ids {
        let token = Token {
            task_id: id,
            trigger_datetime,
        };
        updates::send_token_update(app.get_channel(), ProcessToken::Clear(token)).await?;
    }

    let body = ClearTokens {
        tokens_cleared: task_ids.len() as u64,
    };

    Ok(Json(body).into_response())
}
