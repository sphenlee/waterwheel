use anyhow::Result;
use cadence::StatsdClient;
use lapin::Connection;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::{
    amqp::amqp_connect,
    config,
    config::Config,
    counter::Counter,
    logging, metrics,
    server::jwt,
    util::{spawn_or_crash, spawn_retry},
};

mod config_cache;
mod docker;
pub mod engine;
pub mod env;
mod heartbeat;
mod kube;
mod kubejob;
mod work;

// TODO - move these statics
static WORKER_ID: Lazy<Uuid> = Lazy::new(Uuid::new_v4);

pub static RUNNING_TASKS: Counter = Counter::new();
pub static TOTAL_TASKS: Counter = Counter::new();

pub struct Worker {
    pub amqp_conn: Connection,
    //pub post_office: PostOffice,
    pub statsd: StatsdClient,
    pub config: Config,
}

impl Worker {
    pub async fn new() -> Result<Self> {
        let config = config::load()?;
        logging::setup(&config)?;

        let amqp_conn = amqp_connect(&config).await?;
        let statsd = metrics::new_client(&config)?;

        Ok(Worker {
            amqp_conn,
            statsd,
            config,
        })
    }

    pub async fn run_worker(self) -> Result<!> {
        jwt::load_keys(&self.config)?;

        heartbeat::wait_for_server(&self.config).await;

        let this = Arc::new(self);

        for i in 0..this.config.max_tasks {
            spawn_retry(&format!("worker-{}", i), this.clone(), work::process_work);
        }

        spawn_or_crash(
            "config_updates",
            this.clone(),
            config_cache::process_updates,
        );
        spawn_or_crash("heartbeat", this.clone(), heartbeat::heartbeat);

        info!("worker id {}", *WORKER_ID);

        this.serve().await?;

        unreachable!("worker stopped working");
    }

    async fn serve(self: Arc<Self>) -> Result<()> {
        let mut app = highnoon::App::new(());
        app.at("/")
            .get(|_req| async { Ok("Hello from Waterwheel Worker!") });

        // healthcheck to see if the worker is up
        app.at("/healthcheck").get(|_req| async { Ok("OK") });

        let host = &self.config.worker_bind;
        app.listen(host).await?;

        Ok(())
    }
}
