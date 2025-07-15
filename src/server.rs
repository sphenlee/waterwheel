use crate::{
    amqp::amqp_connect,
    config::Config,
    db, metrics,
    postoffice::PostOffice,
    rendezvous::Rendezvous,
    server::{retries::retry_cluster_changes, triggers::trigger_cluster_changes},
    util::spawn_or_crash,
};
use anyhow::Result;
use api::{jwt, jwt::JwtKeys};
use cadence::StatsdClient;
use chitchat::{Chitchat, ChitchatHandle};
use lapin::Connection;
use sqlx::PgPool;
use std::sync::{Arc, atomic::AtomicUsize};
use tokio::sync::Mutex;
use uuid::Uuid;

pub mod api;
pub mod body_parser;
mod cluster;
mod execute;
mod heartbeat;
mod progress;
mod requeue;
mod retries;
pub mod tokens;
mod trigger_time;
pub mod triggers;
mod updates;

pub struct Server {
    pub scheduler_id: Uuid,
    pub node_id: String,
    pub db_pool: PgPool,
    pub amqp_conn: Connection,
    pub post_office: PostOffice,
    pub statsd: Arc<StatsdClient>,
    pub config: Config,
    pub jwt_keys: JwtKeys,
    pub cluster: ChitchatHandle,
    pub on_cluster_membership_change: tokio::sync::watch::Sender<Rendezvous<String>>,
    pub queued_triggers: AtomicUsize,
    pub waiting_for_trigger_id: Mutex<Option<Uuid>>,
}

impl Server {
    pub async fn new(config: Config) -> Result<Arc<Self>> {
        let db_pool = db::create_pool(&config).await?;
        let amqp_conn = amqp_connect(&config).await?;
        let statsd = metrics::new_client(&config)?;
        let jwt_keys = jwt::load_keys(&config)?;
        let node_id = cluster::get_node_id()?;
        let chitchat = cluster::start_cluster(&config, &node_id).await?;
        let (tx, _rx) = tokio::sync::watch::channel(Rendezvous::new());

        Ok(Arc::new(Server {
            scheduler_id: Uuid::new_v4(),
            node_id,
            db_pool,
            amqp_conn,
            post_office: PostOffice::open(),
            statsd,
            config,
            jwt_keys,
            cluster: chitchat,
            on_cluster_membership_change: tx,
            queued_triggers: AtomicUsize::new(0),
            waiting_for_trigger_id: Mutex::default(),
        }))
    }

    pub async fn run_scheduler(self: Arc<Self>) -> Result<!> {
        spawn_or_crash("heartbeat", self.clone(), heartbeat::heartbeat);
        spawn_or_crash("triggers", self.clone(), triggers::process_triggers);
        spawn_or_crash("tokens", self.clone(), tokens::process_tokens);
        spawn_or_crash("executions", self.clone(), execute::process_executions);
        spawn_or_crash("progress", self.clone(), progress::process_progress);
        spawn_or_crash(
            "trigger_updates",
            self.clone(),
            updates::process_trigger_updates,
        );
        spawn_or_crash(
            "trigger_cluster_changes",
            self.clone(),
            trigger_cluster_changes,
        );
        spawn_or_crash(
            "token_updates",
            self.clone(),
            updates::process_token_updates,
        );
        spawn_or_crash("process_requeue", self.clone(), requeue::process_requeue);
        spawn_or_crash("retry_cluster_changes", self.clone(), retry_cluster_changes);
        spawn_or_crash("process_retries", self.clone(), retries::process_retries);

        // this much be launched last - otherwise other tasks can miss the initial cluster
        // membership change event
        spawn_or_crash("watch_live_nodes", self.clone(), cluster::watch_live_nodes);

        api::serve(self.config.clone()).await?;

        unreachable!("server stop serving");
    }

    pub async fn get_chitchat(self: &Arc<Self>) -> Arc<Mutex<Chitchat>> {
        self.cluster.chitchat()
    }
}
