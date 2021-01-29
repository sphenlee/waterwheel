use crate::postoffice;
use crate::{amqp, spawn_retry};
use anyhow::Result;

use kv_log_macro::info;
use once_cell::sync::Lazy;
use std::sync::atomic::AtomicU64;
use uuid::Uuid;

mod docker;
mod heartbeat;
mod work;

static WORKER_ID: Lazy<Uuid> = Lazy::new(|| Uuid::new_v4());

pub static RUNNING_TASKS: AtomicU64 = AtomicU64::new(0);
pub static TOTAL_TASKS: AtomicU64 = AtomicU64::new(0);

pub async fn run_worker() -> Result<()> {
    amqp::amqp_connect().await?;
    postoffice::open()?;

    let max_tasks = std::env::var("WATERWHEEL_MAX_TASKS")?.parse::<u32>()?;

    for i in 0..max_tasks {
        spawn_retry(&format!("worker-{}", i), work::process_work);
    }

    let mut app = highnoon::App::new(());
    //app.with(tide::log::LogMiddleware::new());
    app.at("/")
        .get(|_req| async { Ok("Hello from Waterwheel Worker!") });

    let host = std::env::var("WATERWHEEL_WORKER_ADDR").unwrap_or_else(|_| "127.0.0.1:0".to_owned());

    info!("worker id {}", *WORKER_ID);

    let addr = host.parse()?;
    /*let tcp = TcpListener::bind(host).await?;
    let addr = tcp.local_addr()?;
    info!("worker listening on {}", addr);*/

    spawn_retry("heartbeat", move || heartbeat::heartbeat(addr));

    app.listen(addr).await?;

    Ok(())
}
