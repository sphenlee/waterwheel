use crate::server::api::types::{period_from_string, Job, Trigger};
use anyhow::Result;
use sqlx::{Postgres, Transaction};
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
