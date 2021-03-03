use std::sync::atomic::AtomicU64;

use anyhow::Result;
use kv_log_macro::info;
use once_cell::sync::Lazy;
use uuid::Uuid;

use crate::amqp;
use crate::config;
use crate::postoffice;
use crate::server::stash;
use crate::util::spawn_retry;

mod docker;
pub mod env;
mod heartbeat;
mod kube;
mod work;

static WORKER_ID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);

pub static RUNNING_TASKS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_TASKS: AtomicU64 = AtomicU64::new(0);

const DEFAULT_MAX_TASKS: u32 = 8;

pub async fn run_worker() -> Result<()> {
    stash::load_keys()?;

    amqp::amqp_connect().await?;
    postoffice::open()?;

    let max_tasks: u32 = config::get_or("WATERWHEEL_MAX_TASKS", DEFAULT_MAX_TASKS);

    for i in 0..max_tasks {
        spawn_retry(&format!("worker-{}", i), work::process_work);
    }

    let mut app = highnoon::App::new(());
    app.at("/")
        .get(|_req| async { Ok("Hello from Waterwheel Worker!") });

    // healthcheck to see if the worker is up
    app.at("/healthcheck").get(|_req| async { Ok("OK") });

    let host: String = config::get_or("WATERWHEEL_WORKER_BIND", "127.0.0.1:0");

    info!("worker id {}", *WORKER_ID);

    /*let tcp = TcpListener::bind(host).await?;
    let addr = tcp.local_addr()?;
    info!("worker listening on {}", host);*/

    spawn_retry("heartbeat", heartbeat::heartbeat);

    app.listen(host).await?;

    Ok(())
}
