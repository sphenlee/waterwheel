use crate::{
    amqp::amqp_connect, config::Config, db, metrics, postoffice::PostOffice, util::spawn_or_crash,
};
use anyhow::Result;
use api::{jwt, jwt::JwtKeys};
use cadence::StatsdClient;
use lapin::Connection;
use sqlx::PgPool;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use chitchat::{Chitchat, ChitchatHandle};
use tokio::sync::Mutex;
use tracing::warn;
use uuid::Uuid;

pub mod api;
pub mod body_parser;
mod execute;
mod progress;
pub mod tokens;
mod trigger_time;
pub mod triggers;
mod updates;
mod cluster;
mod heartbeat;

pub struct Server {
    pub scheduler_id: Uuid,
    pub db_pool: PgPool,
    pub amqp_conn: Connection,
    pub post_office: PostOffice,
    pub statsd: Arc<StatsdClient>,
    pub config: Config,
    pub jwt_keys: JwtKeys,
    pub cluster: Option<ChitchatHandle>,
    pub queued_triggers: AtomicUsize,
    pub waiting_for_trigger_id: Mutex<Option<Uuid>>,
}

impl Server {
    pub async fn new(config: Config) -> Result<Arc<Self>> {
        let db_pool = db::create_pool(&config).await?;
        let amqp_conn = amqp_connect(&config).await?;
        let statsd = metrics::new_client(&config)?;
        let jwt_keys = jwt::load_keys(&config)?;

        Ok(Arc::new(Server {
            scheduler_id: Uuid::new_v4(),
            db_pool,
            amqp_conn,
            post_office: PostOffice::open(),
            statsd,
            config,
            jwt_keys,
            cluster: None,
            queued_triggers: AtomicUsize::new(0),
            waiting_for_trigger_id: Mutex::default(),
        }))
    }

    pub async fn run_scheduler(mut self: Arc<Self>) -> Result<!> {
        cluster::start_cluster(&mut self).await?;

        spawn_or_crash("heartbeat", self.clone(), heartbeat::heartbeat);
        spawn_or_crash("triggers", self.clone(), triggers::process_triggers);
        spawn_or_crash("tokens", self.clone(), tokens::process_tokens);
        spawn_or_crash("executions", self.clone(), execute::process_executions);
        spawn_or_crash("progress", self.clone(), progress::process_progress);
        spawn_or_crash("trigger_updates", self.clone(), updates::process_trigger_updates);
        spawn_or_crash("token_updates", self.clone(), updates::process_token_updates);

        self.run_api().await
    }

    pub async fn run_api(self: Arc<Self>) -> Result<!> {
        if self.config.no_authz {
            warn!("authorization is disabled, this is not recommended in production");
        }

        api::serve(self).await?;

        unreachable!("server stop serving");
    }

    pub async fn get_chitchat(self: &Arc<Self>) -> Arc<Mutex<Chitchat>> {
        self.cluster.as_ref().expect("cluster is not created!").chitchat()
    }
}
