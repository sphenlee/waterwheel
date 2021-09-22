use crate::server::api::request_ext::RequestExt;
use crate::server::api::types::{period_from_string, Job, Trigger};
use crate::server::api::{State, auth};
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder};
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use std::str::FromStr;
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum TriggerError {
    #[error("No schedule - either period or cron must be provided")]
    NoSchedule,
    #[error("Multiple schedule - cannot specify both cron and period")]
    MultipleSchedule,
    #[error("{0}")]
    InvalidCron(cron::error::Error),
    #[error("Period is not valid: {0}")]
    InvalidPeriod(humantime::DurationError),
}

fn bad_req(err: TriggerError) -> highnoon::Result<()> {
    Err(highnoon::Error::bad_request(err.to_string()))
}

pub async fn create_trigger(
    txn: &mut Transaction<'_, Postgres>,
    job: &Job,
    trigger: &Trigger,
) -> highnoon::Result<Uuid> {
    match (&trigger.period, &trigger.cron) {
        (Some(_), Some(_)) => bad_req(TriggerError::MultipleSchedule)?,
        (Some(p), None) => {
            if let Err(e) = humantime::parse_duration(p) {
                bad_req(TriggerError::InvalidPeriod(e))?
            }
        }
        (None, Some(c)) => {
            if let Err(e) = cron::Schedule::from_str(c) {
                bad_req(TriggerError::InvalidCron(e))?
            }
        }
        (None, None) => bad_req(TriggerError::NoSchedule)?,
    };

    let new_id = Uuid::new_v4();

    let (id,) = sqlx::query_as(
        "INSERT INTO trigger(id, name, job_id,
            start_datetime, end_datetime,
            earliest_trigger_datetime, latest_trigger_datetime,
            period, cron, trigger_offset)
        VALUES ($1, $2, $3,
            $4, $5,
            NULL, NULL,
            $6, $7, $8)
        ON CONFLICT(name, job_id)
        DO UPDATE
        SET start_datetime = $4,
            end_datetime = $5,
            period = $6,
            cron = $7,
            trigger_offset = $8
        RETURNING id",
    )
    .bind(&new_id)
    .bind(&trigger.name)
    .bind(&job.uuid)
    .bind(&trigger.start)
    .bind(&trigger.end)
    .bind(period_from_string(&trigger.period)?)
    .bind(&trigger.cron)
    .bind(period_from_string(&trigger.offset)?)
    .fetch_one(&mut *txn)
    .await?;

    // TODO - delete removed triggers

    Ok(id)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct GetTriggerByJob {
    pub trigger_id: Uuid,
    pub trigger_name: String,
    pub start_datetime: DateTime<Utc>,
    pub end_datetime: Option<DateTime<Utc>>,
    pub earliest_trigger_datetime: Option<DateTime<Utc>>,
    pub latest_trigger_datetime: Option<DateTime<Utc>>,
    pub period: Option<i64>, // seconds
    pub cron: Option<String>,
    pub trigger_offset: Option<String>,
}

pub async fn get_triggers_by_job(req: Request<State>) -> highnoon::Result<impl Responder> {
    let job_id = req.param("id")?.parse::<Uuid>()?;

    auth::get().job(job_id, None).check(&req).await?;

    let triggers: Vec<GetTriggerByJob> = sqlx::query_as(
        "SELECT
            id AS trigger_id,
            name AS trigger_name,
            start_datetime,
            end_datetime,
            earliest_trigger_datetime,
            latest_trigger_datetime,
            period,
            cron,
            trigger_offset
        FROM trigger
        WHERE job_id = $1
        ORDER BY latest_trigger_datetime DESC",
    )
    .bind(&job_id)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(triggers))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct GetTrigger {
    pub trigger_id: Uuid,
    pub trigger_name: String,
    pub job_id: Uuid,
    pub job_name: String,
    pub project_id: Uuid,
    pub project_name: String,
}

pub async fn get_trigger(req: Request<State>) -> highnoon::Result<impl Responder> {
    let trigger_id = req.param("id")?.parse::<Uuid>()?;

    // TODO auth check

    let triggers: Option<GetTrigger> = sqlx::query_as(
        "SELECT
            g.id AS trigger_id,
            g.name AS trigger_name,
            j.name AS job_name,
            j.id AS job_id,
            p.name AS project_name,
            p.id AS project_id
        FROM trigger g
        JOIN job j ON j.id = g.job_id
        JOIN project p ON p.id = j.project_id
        WHERE g.id = $1",
    )
    .bind(&trigger_id)
    .fetch_optional(&req.get_pool())
    .await?;

    Ok(Json(triggers))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct GetTriggerTimes {
    trigger_datetime: DateTime<Utc>,
    name: String,
    success: i64,
    active: i64,
    failure: i64,
    waiting: i64,
}

pub async fn get_trigger_times(req: Request<State>) -> highnoon::Result<impl Responder> {
    let trigger_id = req.param("id")?.parse::<Uuid>()?;

    // TODO auth check

    let triggers: Vec<GetTriggerTimes> = sqlx::query_as(
        "WITH these_triggers AS (
            SELECT
                k.trigger_datetime AS trigger_datetime,
                g.name AS name
            FROM trigger g
            JOIN trigger_edge te ON g.id = te.trigger_id
            JOIN token k ON k.task_id = te.task_id
            WHERE g.id = $1
        ),
        these_tokens AS (
            SELECT
                x.trigger_datetime AS trigger_datetime,
                x.name AS name,
                k.state AS state
            FROM these_triggers x
            JOIN token k ON k.trigger_datetime = x.trigger_datetime
            JOIN task t ON t.id = k.task_id
            JOIN trigger g ON g.job_id = t.job_id
            WHERE g.id = $1
        )
        SELECT
            trigger_datetime,
            name,
            sum(CASE WHEN state = 'success' THEN 1 ELSE 0 END) AS success,
            sum(CASE WHEN state = 'active' THEN 1 ELSE 0 END) AS active,
            sum(CASE WHEN state = 'failure' THEN 1 ELSE 0 END) AS failure,
            sum(CASE WHEN state = 'waiting' THEN 1 ELSE 0 END) AS waiting
        FROM these_tokens
        GROUP BY trigger_datetime,
            name
        ORDER BY trigger_datetime DESC
        LIMIT 100
        ",
    )
    .bind(&trigger_id)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(triggers))
}
