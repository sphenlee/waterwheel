use crate::{
    messages::WorkerHeartbeat,
    server::api::{State, request_ext::RequestExt},
};
use highnoon::{Request, Responder, StatusCode};
use tracing::trace;

pub async fn post(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let beat: WorkerHeartbeat = req.body_json().await?;

    // TODO - should heartbeats be JWT protected?

    trace!(uuid=?beat.uuid, "received heartbeat");

    sqlx::query(
        "INSERT INTO worker(
            id,
            addr,
            last_seen_datetime,
            running_tasks,
            total_tasks,
            version
        )
        VALUES($1, $2, $3, $4, $5, $6)
        ON CONFLICT(id)
        DO UPDATE
        SET addr = $2,
            last_seen_datetime = $3,
            running_tasks = $4,
            total_tasks = $5,
            version = $6",
    )
    .bind(beat.uuid)
    .bind(&beat.addr)
    .bind(beat.last_seen_datetime)
    .bind(beat.running_tasks)
    .bind(beat.total_tasks)
    .bind(&beat.version)
    .execute(&req.get_pool())
    .await?;

    Ok(StatusCode::OK)
}
