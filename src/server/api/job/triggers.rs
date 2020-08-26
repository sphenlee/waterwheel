use crate::server::api::types::{period_from_string, Job, Trigger};
use crate::server::api::util::RequestExt;
use crate::server::api::State;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use tide::{Body, Request, Response, StatusCode};
use uuid::Uuid;

pub async fn create_trigger(
    txn: &mut Transaction<'_, Postgres>,
    job: &Job,
    trigger: &Trigger,
) -> Result<Uuid> {
    let trigger_id: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id
        FROM trigger
        WHERE name = $1
        AND job_id = $2",
    )
    .bind(&trigger.name)
    .bind(&job.uuid)
    .fetch_optional(&mut *txn)
    .await?;

    let id = if let Some((id,)) = trigger_id {
        sqlx::query(
            "UPDATE trigger
            SET start_datetime = $1,
                end_datetime = $2,
                period = $3
            WHERE id = $4",
        )
        .bind(&trigger.start)
        .bind(&trigger.end)
        .bind(period_from_string(&trigger.period)?)
        .bind(&id)
        .execute(&mut *txn)
        .await?;

        id
    } else {
        let new_id = Uuid::new_v4();

        sqlx::query(
            "INSERT INTO trigger(id, name, job_id, start_datetime, end_datetime,
                earliest_trigger_datetime, latest_trigger_datetime, period)
            VALUES ($1, $2, $3, $4, $5,
                NULL, NULL, $6)",
        )
        .bind(&new_id)
        .bind(&trigger.name)
        .bind(&job.uuid)
        .bind(&trigger.start)
        .bind(&trigger.end)
        .bind(period_from_string(&trigger.period)?)
        .execute(&mut *txn)
        .await?;

        new_id
    };

    // TODO - delete removed triggers

    Ok(id)
}

#[derive(Serialize, sqlx::FromRow)]
pub struct GetTrigger {
    pub trigger_id: Uuid,
    pub trigger_name: String,
    pub start_datetime: DateTime<Utc>,
    pub end_datetime: Option<DateTime<Utc>>,
    pub earliest_trigger_datetime: Option<DateTime<Utc>>,
    pub latest_trigger_datetime: Option<DateTime<Utc>>,
    pub period: i64, // seconds
    pub offset: Option<String>,
}

pub async fn get_triggers(req: Request<State>) -> tide::Result {
    let job_id = req.param::<Uuid>("id")?;

    let triggers = sqlx::query_as::<_, GetTrigger>(
        "SELECT
            id AS trigger_id,
            name AS trigger_name,
            start_datetime,
            end_datetime,
            earliest_trigger_datetime,
            latest_trigger_datetime,
            period,
            NULL AS \"offset\"
        FROM trigger
        WHERE job_id = $1
        ORDER BY latest_trigger_datetime DESC",
    )
    .bind(&job_id)
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Response::builder(StatusCode::Ok)
        .body(Body::from_json(&triggers)?)
        .build())
}
