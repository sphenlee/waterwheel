use crate::{
    amqp::amqp_connect, config::Config, db, metrics, postoffice::PostOffice,
    util::spawn_or_crash,
};
use anyhow::Result;
use cadence::StatsdClient;
use lapin::Connection;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::warn;

pub mod api;
pub mod body_parser;
mod execute;
pub mod jwt;
mod progress;
pub mod tokens;
mod trigger_time;
pub mod triggers;
mod updates;

pub struct Server {
    pub db_pool: PgPool,
    pub amqp_conn: Connection,
    pub post_office: PostOffice,
    pub statsd: StatsdClient,
    pub config: Config,
}

impl Server {
    pub async fn new(config: Config) -> Result<Self> {
        let db_pool = db::create_pool(&config).await?;
        let amqp_conn = amqp_connect(&config).await?;
        let statsd = metrics::new_client(&config)?;

        Ok(Server {
            db_pool,
            amqp_conn,
            post_office: PostOffice::open(),
            statsd,
            config,
        })
    }

    pub async fn run_scheduler(self) -> Result<!> {
        let this = Arc::new(self);

        spawn_or_crash("triggers", this.clone(), triggers::process_triggers);
        spawn_or_crash("tokens", this.clone(), tokens::process_tokens);
        spawn_or_crash("executions", this.clone(), execute::process_executions);
        spawn_or_crash("progress", this.clone(), progress::process_progress);
        spawn_or_crash("updates", this.clone(), updates::process_updates);

        this.run_api_inner().await
    }

    pub async fn run_api(self) -> Result<!> {
        Arc::new(self).run_api_inner().await
    }

    async fn run_api_inner(self: Arc<Self>) -> Result<!> {
        if self.config.no_authz {
            warn!("authorization is disabled, this is not recommended in production");
        }

        jwt::load_keys(&self.config)?;

        api::serve(self).await?;

        unreachable!("server stop serving");
    }
}
