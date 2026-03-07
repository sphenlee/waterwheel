use crate::server::api::{App, AppResult, auth};
use axum::{
    Json,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
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

#[axum::debug_handler]
pub async fn list(State(app): State<App>, headers: HeaderMap) -> AppResult<Response> {
    auth::list(&app).kind("schedulers").check(&headers).await?;

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
    .fetch_all(&app.get_pool())
    .await?;

    Ok(Json(schedulers).into_response())
}
