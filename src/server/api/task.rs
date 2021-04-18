use crate::messages::{SchedulerUpdate, TaskDef, TaskPriority, Token};
use crate::server::api::request_ext::RequestExt;
use crate::server::api::{updates, State};
use crate::server::tokens::ProcessToken;
use futures::TryStreamExt;
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder, StatusCode};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

pub async fn clear_token(req: Request<State>) -> highnoon::Result<impl Responder> {
    let task_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;

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
    .bind(&token.task_id)
    .bind(&token.trigger_datetime)
    .execute(&mut txn)
    .await?;

    updates::send(
        req.get_channel(),
        SchedulerUpdate::ProcessToken(ProcessToken::Activate(token, TaskPriority::High)),
    )
    .await?;

    txn.commit().await?;

    Ok(StatusCode::CREATED)
}

#[derive(Deserialize)]
struct ClearTokenParams {
    first: Option<DateTime<Utc>>,
    last: Option<DateTime<Utc>>,
    only_failed: Option<bool>,
}

#[derive(Serialize)]
struct ClearTokenReply {
    cleared: u64,
}


pub async fn clear_multiple_tokens(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let task_id = req.param("id")?.parse::<Uuid>()?;
    let params: ClearTokenParams = req.body_json().await?;

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
    .bind(&task_id)
    .bind(&params.first)
    .bind(&params.last)
    .bind(params.only_failed.unwrap_or(false))
    .fetch(&mut txn);

    let mut count = 0u64;
    while let Some((trigger_datetime,)) = cursor.try_next().await? {
        count += 1;

        let token = Token {
            task_id,
            trigger_datetime
        };

        updates::send(
            req.get_channel(),
            SchedulerUpdate::ProcessToken(ProcessToken::Activate(token, TaskPriority::High)),
        )
        .await?;
    }

    drop(cursor);

    txn.commit().await?;

    Ok(Json(ClearTokenReply {
        cleared: count
    }))
}

pub async fn get_task_def(req: Request<State>) -> highnoon::Result<impl Responder> {
    let task_id = req.param("id")?.parse::<Uuid>()?;

    let def: Option<TaskDef> = sqlx::query_as(
        "SELECT
                t.id AS task_id,
                t.name AS task_name,
                j.id AS job_id,
                j.name AS job_name,
                p.id AS project_id,
                p.name AS project_name,
                image,
                COALESCE(args, ARRAY[]::VARCHAR[]) AS args,
                env
            FROM task t
            JOIN job j on t.job_id = j.id
            JOIN project p ON j.project_id = p.id
            WHERE t.id = $1",
    )
    .bind(&task_id)
    .fetch_optional(&req.get_pool())
    .await?;

    Ok(def.map(Json))
}
