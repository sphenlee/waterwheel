use anyhow::Result;
use once_cell::sync::Lazy;
use tracing::info;
use uuid::Uuid;

use crate::amqp;
use crate::config;
use crate::counter::Counter;
use crate::server::jwt;
use crate::util::{spawn_or_crash, spawn_retry};

mod config_cache;
mod docker;
pub mod env;
mod heartbeat;
mod kube;
mod kubejob;
mod work;
pub mod engine;


static WORKER_ID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);

pub static RUNNING_TASKS: Counter = Counter::new();
pub static TOTAL_TASKS: Counter = Counter::new();


pub async fn run_worker() -> Result<()> {
    jwt::load_keys()?;

    amqp::amqp_connect().await?;

    let max_tasks: u32 = config::get().max_tasks;

    heartbeat::wait_for_server().await;

    for i in 0..max_tasks {
        spawn_retry(&format!("worker-{}", i), work::process_work);
    }

    spawn_or_crash("config_updates", config_cache::process_updates);
    spawn_or_crash("heartbeat", heartbeat::heartbeat);

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
