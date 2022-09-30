use crate::server::api::{auth, request_ext::RequestExt, State};
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder};
use serde::{Serialize};
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow)]
struct SchedulerState {
    pub uuid: Uuid,
    pub version: String,
    pub last_seen_datetime: DateTime<Utc>,
    pub queued_triggers: i32,
    pub waiting_for_trigger_id: Option<Uuid>,
    pub waiting_for_trigger_job_id: Option<Uuid>,
    pub status: String,
}

pub async fn list(req: Request<State>) -> highnoon::Result<impl Responder> {
    auth::list().kind("schedulers").check(&req).await?;

    let schedulers: Vec<SchedulerState> = sqlx::query_as(
        "SELECT
            s.id AS uuid,
            s.version,
            s.last_seen_datetime,
            s.queued_triggers,
            s.waiting_for_trigger_id,
            g.job_id AS waiting_for_trigger_job_id,
            CASE
                WHEN CURRENT_TIMESTAMP - s.last_seen_datetime > INTERVAL '1 minute' THEN 'gone'
                ELSE 'up'
            END AS status
        FROM scheduler s
        LEFT JOIN trigger g ON s.waiting_for_trigger_id = g.id
        WHERE CURRENT_TIMESTAMP - s.last_seen_datetime < INTERVAL '1 hour'
        ORDER BY s.last_seen_datetime DESC",
    )
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(schedulers))
}
