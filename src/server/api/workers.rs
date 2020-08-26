use crate::server::api::State;
use crate::server::heartbeat::WORKER_STATUS;
use tide::{Body, Request, Response, StatusCode};

pub async fn list(_req: Request<State>) -> tide::Result {
    let status = WORKER_STATUS.lock().await;

    let workers: Vec<_> = status.values().cloned().collect();

    Ok(Response::builder(StatusCode::Ok)
        .body(Body::from_json(&workers)?)
        .build())
}
