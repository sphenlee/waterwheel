use std::sync::atomic::AtomicI32;

use anyhow::Result;
use kv_log_macro::info;
use once_cell::sync::Lazy;
use uuid::Uuid;

use crate::amqp;
use crate::config;
use crate::server::jwt;
use crate::util::spawn_retry;
use std::str::FromStr;

mod config_cache;
mod docker;
pub mod env;
mod heartbeat;
mod kube;
mod work;

static WORKER_ID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);

pub static RUNNING_TASKS: AtomicI32 = AtomicI32::new(0);
pub static TOTAL_TASKS: AtomicI32 = AtomicI32::new(0);

#[derive(Copy, Clone, serde::Deserialize)]
#[serde(rename_all="lowercase")]
pub enum TaskEngine {
    /// Null engine always returns success - disabled in release builds
    #[cfg(debug_assertions)]
    Null,
    /// Use a local docker instance (TODO - allow remote docker)
    Docker,
    /// Use a remote Kubernetes cluster
    Kubernetes,
}

impl FromStr for TaskEngine {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(debug_assertions)]
            "null" => Ok(TaskEngine::Null),
            "docker" => Ok(TaskEngine::Docker),
            "kubernetes" => Ok(TaskEngine::Kubernetes),
            _ => Err(anyhow::Error::msg(
                "invalid engine, valid options: docker, kubernetes",
            )),
        }
    }
}

pub async fn run_worker() -> Result<()> {
    jwt::load_keys()?;

    amqp::amqp_connect().await?;

    let max_tasks: u32 = config::get().max_tasks;

    for i in 0..max_tasks {
        spawn_retry(&format!("worker-{}", i), work::process_work);
    }

    spawn_retry("config_updates", config_cache::process_updates);
    spawn_retry("heartbeat", heartbeat::heartbeat);

    info!("worker id {}", *WORKER_ID);

    serve().await?;

    Ok(())
}

async fn serve() -> Result<()> {
    let mut app = highnoon::App::new(());
    app.at("/")
        .get(|_req| async { Ok("Hello from Waterwheel Worker!") });

    // healthcheck to see if the worker is up
    app.at("/healthcheck").get(|_req| async { Ok("OK") });

    let host = &config::get().worker_bind;
    app.listen(host).await?;

    Ok(())
}
