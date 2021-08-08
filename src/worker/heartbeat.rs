use crate::config;
use crate::messages::WorkerHeartbeat;
use anyhow::Result;

use chrono::Utc;
use tracing::{debug, trace, warn};

use super::{RUNNING_TASKS, TOTAL_TASKS, WORKER_ID};
use reqwest::{StatusCode, Url};
use std::sync::atomic::Ordering;

pub async fn heartbeat() -> Result<!> {
    let server_addr = config::get().server_addr.as_ref();
    let url = Url::parse(server_addr)?.join("api/heartbeat")?;

    let client = reqwest::Client::new();

    loop {
        trace!("posting heartbeat");

        let res = client
            .post(url.clone())
            .json(&WorkerHeartbeat {
                uuid: *WORKER_ID,
                addr: "TODO".to_owned(),
                last_seen_datetime: Utc::now(),
                running_tasks: RUNNING_TASKS.load(Ordering::Relaxed),
                total_tasks: TOTAL_TASKS.load(Ordering::Relaxed),
            })
            .send()
            .await;

        match res {
            Ok(resp) if resp.status() == StatusCode::OK => {
                trace!("heartbeat: {}", resp.status())
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await?;
                warn!("heartbeat: {}", status);
                debug!("heartbeat: {}", body);
            }
            Err(err) => {
                warn!("failed to send heartbeat to the server: {}", err)
            }
        };

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}
