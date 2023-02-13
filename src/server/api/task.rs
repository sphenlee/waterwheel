use crate::{
    messages::{ProcessToken, TaskDef, TaskPriority, Token},
    server::api::{auth, jwt, request_ext::RequestExt, updates, State},
};
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use highnoon::{Json, Request, Responder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
struct ActivateTokenParams {
    priority: Option<TaskPriority>,
}

pub async fn activate_token(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let task_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;
    let params: ActivateTokenParams = req.body_json().await?;

    // TODO auth check

    let token = Token {
        task_id,
        trigger_datetime,
    };

    let pool = req.get_pool();
    let mut txn = pool.begin().await?;

    sqlx::query(
        "INSERT INTO token(task_id, trigger_datetime, count, state)
            VALUES ($1, $2, (SELECT threshold FROM task WHERE id = $1), 'waiting')
            ON CONFLICT(task_id, trigger_datetime)
            DO UPDATE
            SET count = (SELECT threshold FROM task WHERE id = $1),
                state = 'waiting'",
    )
    .bind(token.task_id)
    .bind(token.trigger_datetime)
    .execute(&mut txn)
    .await?;

    let priority = params.priority.unwrap_or(TaskPriority::High);

    updates::send_token_update(req.get_channel(), ProcessToken::Activate(token, priority)).await?;

    txn.commit().await?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize)]
struct ActivateMultipleTokensParams {
    priority: Option<TaskPriority>,
    first: Option<DateTime<Utc>>,
    last: Option<DateTime<Utc>>,
    only_failed: Option<bool>,
}

#[derive(Serialize)]
struct ActivateTokenReply {
    cleared: u64,
}

pub async fn activate_multiple_tokens(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let task_id = req.param("id")?.parse::<Uuid>()?;
    let params: ActivateMultipleTokensParams = req.body_json().await?;

    if params.first.is_none() && params.last.is_none() && params.only_failed.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            "one or more of 'first','last' and 'only_failed' must be specified",
        )
            .into_response();
    }

    // TODO auth check

    let pool = req.get_pool();
    let mut txn = pool.begin().await?;

    let mut cursor = sqlx::query_as(
        "UPDATE token
         SET count = (SELECT threshold FROM task WHERE id = $1),
             state = 'waiting'
         WHERE task_id = $1
         AND ($2 IS NULL OR trigger_datetime > $2)
         AND ($3 IS NULL OR trigger_datetime < $3)
         AND (NOT $4 OR state = 'failure')
         RETURNING trigger_datetime",
    )
    .bind(task_id)
    .bind(params.first)
    .bind(params.last)
    .bind(params.only_failed.unwrap_or(false))
    .fetch(&mut txn);

    let priority = params.priority.unwrap_or(TaskPriority::BackFill);

    let mut count = 0u64;
    while let Some((trigger_datetime,)) = cursor.try_next().await? {
        count += 1;

        let token = Token {
            task_id,
            trigger_datetime,
        };

        updates::send_token_update(req.get_channel(), ProcessToken::Activate(token, priority))
            .await?;
    }

    drop(cursor);

    txn.commit().await?;

    Json(ActivateTokenReply { cleared: count }).into_response()
}

pub async fn get_task_def(req: Request<State>) -> highnoon::Result<Response> {
    let maybe_def = get_task_def_common(&req).await?;

    if let Some(def) = maybe_def {
        auth::get()
            .job(def.job_id, def.project_id)
            .kind("task")
            .check(&req)
            .await?;
        Json(def).into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

pub async fn internal_get_task_def(req: Request<State>) -> highnoon::Result<impl Responder> {
    let task_id = req.param("id")?.parse::<Uuid>()?;

    jwt::validate_config_jwt(&req, task_id)?;

    let maybe_def = get_task_def_common(&req).await?;
    Ok(maybe_def.map(Json))
}

async fn get_task_def_common(req: &Request<State>) -> highnoon::Result<Option<TaskDef>> {
    let task_id = req.param("id")?.parse::<Uuid>()?;

    let maybe_def: Option<TaskDef> = sqlx::query_as(
        "SELECT
                t.id AS task_id,
                t.name AS task_name,
                j.id AS job_id,
                j.name AS job_name,
                p.id AS project_id,
                p.name AS project_name,
                image,
                COALESCE(args, ARRAY[]::VARCHAR[]) AS args,
                env,
                j.paused
            FROM task t
            JOIN job j on t.job_id = j.id
            JOIN project p ON j.project_id = p.id
            WHERE t.id = $1",
    )
    .bind(task_id)
    .fetch_optional(&req.get_pool())
    .await?;

    Ok(maybe_def)
}
