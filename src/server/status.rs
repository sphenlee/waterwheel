use async_std::sync::Mutex;
use once_cell::sync::Lazy;
use serde::Serialize;

#[derive(Default, Serialize)]
pub struct ServerStatus {
    pub queued_triggers: usize,
    pub num_workers: usize,
    pub running_tasks: u64,
}

pub static SERVER_STATUS: Lazy<Mutex<ServerStatus>> =
    Lazy::new(|| Mutex::new(ServerStatus::default()));
