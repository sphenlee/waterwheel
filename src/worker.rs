use crate::postoffice;
use crate::{amqp, spawn_and_log};
use anyhow::Result;
use async_std::net::TcpListener;

use kv_log_macro::info;
use once_cell::sync::Lazy;
use uuid::Uuid;

mod docker;
mod heartbeat;
mod work;

static WORKER_ID: Lazy<Uuid> = Lazy::new(|| Uuid::new_v4());

pub async fn run_worker() -> Result<()> {
    amqp::amqp_connect().await?;
    postoffice::open()?;

    let max_tasks = std::env::var("WATERWHEEL_MAX_TASKS")?.parse::<u32>()?;

    for i in 0..max_tasks {
        spawn_and_log(&format!("worker-{}", i), work::process_work());
    }

    let mut app = tide::new();
    app.with(tide::log::LogMiddleware::new());
    app.at("/")
        .get(|_req| async { Ok("Hello from Waterwheel Worker!") });

    let host = std::env::var("WATERWHEEL_WORKER_ADDR").unwrap_or_else(|_| "127.0.0.1:0".to_owned());

    info!("worker id {}", *WORKER_ID);

    let tcp = TcpListener::bind(host).await?;
    let addr = tcp.local_addr()?;
    info!("worker listening on {}", addr);

    spawn_and_log("heartbeat", heartbeat::heartbeat(addr.clone()));

    app.listen(tcp).await?;

    Ok(())
}
