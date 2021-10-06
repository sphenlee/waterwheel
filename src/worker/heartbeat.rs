use crate::{config, GIT_VERSION};
use crate::messages::WorkerHeartbeat;
use anyhow::Result;

use chrono::Utc;
use tracing::{debug, trace, warn, error, info};

use super::{RUNNING_TASKS, TOTAL_TASKS, WORKER_ID};
use reqwest::{StatusCode, Url, Response};

pub async fn post_heartbeat(client: &reqwest::Client) -> Result<Response> {
    let server_addr = config::get().server_addr.as_ref();
    let url = Url::parse(server_addr)?.join("int-api/heartbeat")?;

    let resp = client
        .post(url.clone())
        .json(&WorkerHeartbeat {
            uuid: *WORKER_ID,
            addr: "TODO".to_owned(),
            last_seen_datetime: Utc::now(),
            running_tasks: RUNNING_TASKS.get(),
            total_tasks: TOTAL_TASKS.get(),
            version: GIT_VERSION.to_owned(),
        })
        .send()
        .await?;

    Ok(resp)
}

pub async fn heartbeat() -> Result<!> {
    let client = reqwest::Client::new();

    loop {
        trace!("posting heartbeat");
        match post_heartbeat(&client).await {
            Ok(resp) if resp.status() == StatusCode::OK => {
                trace!("heartbeat: OK");
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await?;
                warn!("heartbeat: {}", status);
                debug!("heartbeat: {}", body);
            }
            Err(err) => {
                warn!("failed to send heartbeat to the server: {}", err);
            }
        };

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

pub async fn wait_for_server() {
    // before accepting tasks perform a synchronous heartbeat to ensure
    // the server has our worker ID recorded
    trace!("waiting for initial heartbeat");
    let mut retries = 5;
    loop {
        trace!("sending heartbeat");
        match post_heartbeat(&reqwest::Client::new()).await {
            Ok(resp) if resp.status() == reqwest::StatusCode::OK => {
                trace!("heartbeat: OK");
                break;
            }
            Ok(resp) => {
                let status = resp.status();
                let body = resp.text().await.expect("error getting response body");
                warn!("heartbeat: {}", status);
                debug!("heartbeat: {}", body);
            }
            Err(err) => {
                info!("waiting for server...");
                debug!("failed to send heartbeat to the server: {}", err);
            }
        }

        retries -= 1;
        if retries == 0 {
            error!("failed to send initial heartbeat to the server, aborting!");
            std::process::exit(1);
        }

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    info!("server received initial heartbeat, starting work");
}
