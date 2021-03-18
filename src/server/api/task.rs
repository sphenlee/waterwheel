use crate::messages::{SchedulerUpdate, TaskDef, TaskPriority, Token};
use crate::server::api::request_ext::RequestExt;
use crate::server::api::{updates, State};
use crate::server::tokens::ProcessToken;
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder, StatusCode};
use uuid::Uuid;

pub async fn create_token(req: Request<State>) -> highnoon::Result<impl Responder> {
    let task_id = req.param("id")?.parse::<Uuid>()?;
    let trigger_datetime = req.param("trigger_datetime")?.parse::<DateTime<Utc>>()?;

    let token = Token {
        task_id,
        trigger_datetime,
    };

    sqlx::query(
        "INSERT INTO token(task_id, trigger_datetime, count, state)
            VALUES ($1, $2, 0, 'waiting')
            ON CONFLICT(task_id, trigger_datetime)
            DO UPDATE SET count = 0",
    )
    .bind(&token.task_id)
    .bind(&token.trigger_datetime)
    .execute(&req.get_pool())
    .await?;

    updates::send(
        req.get_channel(),
        SchedulerUpdate::ProcessToken(ProcessToken::Activate(token, TaskPriority::High)),
    )
    .await?;

    Ok(StatusCode::CREATED)
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
