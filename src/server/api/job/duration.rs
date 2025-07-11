use crate::server::api::{State, auth, request_ext::RequestExt};
use chrono::{DateTime, Utc};
use highnoon::{Json, Request, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct GetDurationQuery {
    before: Option<DateTime<Utc>>,
    limit: Option<i32>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TaskDuration {
    trigger_datetime: DateTime<Utc>,
    duration: Option<f64>, // in seconds, because sqlx doesn't support durations
    task_name: String,
}

#[derive(Serialize)]
pub struct GetDuration {
    pub duration: Vec<TaskDuration>,
}

pub async fn get_duration(req: Request<State>) -> highnoon::Result<impl Responder> {
    let job_id = req.param("id")?.parse::<Uuid>()?;

    let query: GetDurationQuery = req.query()?;

    auth::get().job(job_id, None).check(&req).await?;

    let duration: Vec<TaskDuration> = sqlx::query_as(
        "WITH these_triggers AS (
            SELECT DISTINCT
                r.trigger_datetime AS trigger_datetime
            FROM task_run r
            JOIN task t ON t.id = r.task_id
            WHERE t.job_id = $1
            AND ($2 IS NULL OR r.trigger_datetime < $2)
            ORDER BY r.trigger_datetime DESC
            LIMIT $3
        ),
        these_tasks AS (
            SELECT
                t.id AS id,
                t.name AS name
            FROM task t
            WHERE t.job_id = $1
        )
        SELECT
            t.name AS task_name,
            x.trigger_datetime AS trigger_datetime,
            CAST(EXTRACT(EPOCH FROM MAX(r.finish_datetime - r.started_datetime)) AS FLOAT8)
                AS duration
        FROM these_triggers x
        CROSS JOIN these_tasks t
        LEFT OUTER JOIN task_run r
            ON x.trigger_datetime = r.trigger_datetime
            AND t.id = r.task_id
        WHERE r.state IN ('success', 'failure')
        GROUP BY t.name, x.trigger_datetime
        ORDER BY t.name, x.trigger_datetime
        ",
    )
    .bind(job_id)
    .bind(query.before)
    .bind(query.limit.unwrap_or(31))
    .fetch_all(&req.get_pool())
    .await?;

    Ok(Json(GetDuration { duration }))
}
