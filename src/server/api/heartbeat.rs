use crate::messages::WorkerHeartbeat;
use crate::server::api::request_ext::RequestExt;
use crate::server::api::State;
use highnoon::{Request, Responder, StatusCode};
use kv_log_macro::trace;

pub async fn post(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let beat: WorkerHeartbeat = req.body_json().await?;

    trace!("received heartbeat", {
        uuid: beat.uuid.to_string(),
    });

    sqlx::query(
        "INSERT INTO worker(
            id,
            addr,
            last_seen_datetime,
            running_tasks,
            total_tasks
        )
        VALUES($1, $2, $3, $4, $5)
        ON CONFLICT(id)
        DO UPDATE
        SET addr = $2,
            last_seen_datetime = $3,
            running_tasks = $4,
            total_tasks = $5",
    )
    .bind(&beat.uuid)
    .bind(&beat.addr)
    .bind(&beat.last_seen_datetime)
    .bind(&beat.running_tasks)
    .bind(&beat.total_tasks)
    .execute(&req.get_pool())
    .await?;

    Ok(StatusCode::OK)
}
