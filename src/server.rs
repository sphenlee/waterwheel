use crate::{
    amqp::amqp_connect, config::Config, db, metrics, postoffice::PostOffice, util::spawn_or_crash,
};
use anyhow::Result;
use api::{jwt, jwt::JwtKeys};
use cadence::StatsdClient;
use lapin::Connection;
use sqlx::PgPool;
use std::sync::Arc;
use tracing::warn;

pub mod api;
pub mod body_parser;
mod execute;
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
    pub jwt_keys: JwtKeys,
}

impl Server {
    pub async fn new(config: Config) -> Result<Arc<Self>> {
        let db_pool = db::create_pool(&config).await?;
        let amqp_conn = amqp_connect(&config).await?;
        let statsd = metrics::new_client(&config)?;
        let jwt_keys = jwt::load_keys(&config)?;

        Ok(Arc::new(Server {
            db_pool,
            amqp_conn,
            post_office: PostOffice::open(),
            statsd,
            config,
            jwt_keys,
        }))
    }

    pub async fn run_scheduler(self: Arc<Self>) -> Result<!> {
        spawn_or_crash("triggers", self.clone(), triggers::process_triggers);
        spawn_or_crash("tokens", self.clone(), tokens::process_tokens);
        spawn_or_crash("executions", self.clone(), execute::process_executions);
        spawn_or_crash("progress", self.clone(), progress::process_progress);
        spawn_or_crash("updates", self.clone(), updates::process_updates);

        self.run_api().await
    }

    pub async fn run_api(self: Arc<Self>) -> Result<!> {
        if self.config.no_authz {
            warn!("authorization is disabled, this is not recommended in production");
        }

        api::serve(self).await?;

        unreachable!("server stop serving");
    }
}
