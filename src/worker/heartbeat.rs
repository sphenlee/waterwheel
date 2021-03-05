use crate::messages::WorkerHeartbeat;
use crate::config;
use anyhow::Result;

use chrono::Utc;
use log::{trace, warn};

use super::{RUNNING_TASKS, TOTAL_TASKS, WORKER_ID};
use std::sync::atomic::Ordering;
use reqwest::Url;


pub async fn heartbeat() -> Result<!> {
    let server_addr: String = config::get("WATERWHEEL_SERVER_ADDR")?;
    let url = Url::parse(&server_addr)?.join("/api/heartbeat")?;

    let client = reqwest::Client::new();

    loop {
        trace!("posting heartbeat");

        let res = client.post(url.clone())
            .json(&WorkerHeartbeat {
                uuid: *WORKER_ID,
                addr: "TODO".to_owned(),
                last_seen_datetime: Utc::now(),
                running_tasks: RUNNING_TASKS.load(Ordering::Relaxed),
                total_tasks: TOTAL_TASKS.load(Ordering::Relaxed),
            })
            .send().await;

        match res {
            Ok(resp) => {
                trace!("heartbeat: {}", resp.status())
            },
            Err(err) => {
                warn!("failed to send heartbeat to the server: {}", err)
            },
        };

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
