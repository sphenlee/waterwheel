use crate::messages::WorkerHeartbeat;
use crate::server::api::State;
use crate::server::status::SERVER_STATUS;
use highnoon::{Request, Responder, StatusCode};
use kv_log_macro::trace;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use tokio::sync::Mutex;
use uuid::Uuid;

pub static WORKER_STATUS: Lazy<Mutex<HashMap<Uuid, WorkerHeartbeat>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn post(mut req: Request<State>) -> highnoon::Result<impl Responder> {
    let beat: WorkerHeartbeat = req.body_json().await?;

    trace!("received heartbeat", {
        uuid: beat.uuid.to_string(),
    });

    let num_workers: usize;
    let running_tasks: u64;
    {
        let mut worker_status = WORKER_STATUS.lock().await;
        worker_status.insert(beat.uuid, beat);
        num_workers = worker_status.len();
        running_tasks = worker_status.values().map(|hb| hb.running_tasks).sum();
    }

    {
        let mut server_status = SERVER_STATUS.lock().await;
        server_status.num_workers = num_workers;
        server_status.running_tasks = running_tasks;
    }

    Ok(StatusCode::OK)
}
