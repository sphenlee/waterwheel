use crate::postoffice;
use crate::{amqp, spawn_retry};
use crate::server::stash;
use anyhow::Result;

use kv_log_macro::info;
use once_cell::sync::Lazy;
use std::sync::atomic::AtomicU64;
use uuid::Uuid;

mod docker;
pub mod env;
mod heartbeat;
mod kube;
mod work;

static WORKER_ID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);

pub static RUNNING_TASKS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_TASKS: AtomicU64 = AtomicU64::new(0);

pub async fn run_worker() -> Result<()> {
    stash::load_rsa_keys()?;

    amqp::amqp_connect().await?;
    postoffice::open()?;

    let max_tasks = std::env::var("WATERWHEEL_MAX_TASKS")?.parse::<u32>()?;

    for i in 0..max_tasks {
        spawn_retry(&format!("worker-{}", i), work::process_work);
    }

    let mut app = highnoon::App::new(());
    app.at("/")
        .get(|_req| async { Ok("Hello from Waterwheel Worker!") });

    // healthcheck to see if the worker is up
    app.at("/healthcheck").get(|_req| async { Ok("OK") });

    let host = std::env::var("WATERWHEEL_WORKER_BIND").unwrap_or_else(|_| "127.0.0.1:0".to_owned());

    info!("worker id {}", *WORKER_ID);

    /*let tcp = TcpListener::bind(host).await?;
    let addr = tcp.local_addr()?;
    info!("worker listening on {}", host);*/

    spawn_retry("heartbeat", heartbeat::heartbeat);

    app.listen(host).await?;

    Ok(())
}
