use crate::server::api::State;
use crate::server::heartbeat::WORKER_STATUS;
use hightide::{Json, Responder};
use tide::Request;

pub async fn list(_req: Request<State>) -> impl Responder {
    let status = WORKER_STATUS.lock().await;

    let workers: Vec<_> = status.values().cloned().collect();

    Json(workers)
}
