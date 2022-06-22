///! Scheduler Cluster.
///! Multiple schedulers can form a cluster using the chitchat membership finding library
///! and then choose which triggers to manage using rendezvous hashing.
///! Membership changes are expected to be rare and don't need to be handled quickly.

use anyhow::{Context, Result};
use std::sync::Arc;
use std::collections::HashSet;
use chitchat::{Chitchat, ChitchatConfig, ChitchatHandle, NodeId};
use chitchat::transport::UdpTransport;
use chrono::Utc;
use futures::{Stream, StreamExt as _};
use postage::prelude::{*, Stream as _};
use sqlx::PgPool;
use tokio::sync::Mutex;
use tracing::{debug, info, trace};
use uuid::Uuid;
use crate::config::Config;
use crate::messages::TriggerUpdate;
use crate::server::Server;
use crate::server::triggers::TriggerChange;
use crate::util::{deref, first};

fn get_node_id(config: &Config) -> Result<NodeId> {
    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
    let ts = Utc::now().timestamp();
    let name = format!("{}/{}", hostname, ts);

    let gossip_addr = config.cluster_gossip_addr.parse()
        .context("parsing cluster_gossip_addr")?;

    Ok(NodeId::new(name, gossip_addr))
}

pub async fn start_cluster(server: Arc<Server>) -> Result<()> {
    let node_id = get_node_id(&server.config)?;
    debug!("node id is {:?}", node_id);

    let config = ChitchatConfig {
        node_id,
        cluster_id: server.config.cluster_id.as_deref().unwrap_or("waterwheel").to_owned(),
        listen_addr: server.config.cluster_gossip_bind.parse()
            .context("parsing cluster_gossip_bind")?,
        seed_nodes: server.config.cluster_seed_nodes.clone(),
        ..ChitchatConfig::default()
    };

    let chitchat_handle = chitchat::spawn_chitchat(
            config,
            vec![], &UdpTransport).await?;

    let watcher = chitchat_handle
        .with_chitchat(|cc| {
            cc.live_nodes_watcher()
        }).await;

    tokio::spawn(trigger_updates(server.clone(), chitchat_handle.chitchat()));
    tokio::spawn(watch_live_nodes(server.clone(), chitchat_handle, watcher));

    Ok(())
}

async fn watch_live_nodes<W>(server: Arc<Server>, chitchat_handle: ChitchatHandle, mut watcher: W) -> Result<!>
    where W: Stream<Item=HashSet<NodeId>> + Unpin
{
    let mut change_tx = server.post_office.post_mail::<TriggerChange>().await?;
    let mut current_triggers = HashSet::new();

    while let Some(live_nodes) = watcher.next().await {
        info!("cluster membership changed");
        let triggers = get_all_triggers(&server.db_pool).await?;

        let h = std::hash::BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default();
        let mut rendezvous = hash_rings::rendezvous::Client::with_hasher(h);

        let me = &chitchat_handle.node_id().id;
        rendezvous.insert_node(me, 1);
        for item in &live_nodes {
            rendezvous.insert_node(&item.id, 1);
        }

        for trigger in &triggers {
            rendezvous.insert_point(trigger);
        }

        let new_triggers: HashSet<_> = rendezvous.get_points(me)
            .into_iter()
            .map(deref)
            .collect();

        let to_remove = current_triggers.difference(&new_triggers).map(deref).collect();
        info!("remove trigger {:?}", to_remove);
        change_tx.send(TriggerChange::Remove(to_remove)).await?;

        let to_add = new_triggers.difference(&current_triggers).map(deref).collect();
        info!("add trigger {:?}", to_add);
        change_tx.send(TriggerChange::Add(to_add)).await?;

        current_triggers = new_triggers;

        dbg!(&current_triggers);
    }

    unreachable!("chitchat watcher was closed!");
}

async fn trigger_updates(server: Arc<Server>, chitchat: Arc<Mutex<Chitchat>>) -> Result<!> {
    let mut trigger_rx = server.post_office.receive_mail::<TriggerUpdate>().await?;

    let mut change_tx = server.post_office.post_mail::<TriggerChange>().await?;

    while let Some(TriggerUpdate(uuids)) = trigger_rx.recv().await {
        trace!(?uuids, "got trigger update");

        let h = std::hash::BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default();
        let mut rendezvous = hash_rings::rendezvous::Client::with_hasher(h);

        let chitchat = chitchat.lock().await;
        let me = &chitchat.self_node_id().id;
        rendezvous.insert_node(me, 1);
        for item in chitchat.live_nodes() {
            rendezvous.insert_node(&item.id, 1);
        }

        let to_add = uuids.iter().filter(|trigger| {
            rendezvous.get_node(trigger) == &chitchat.self_node_id().id
        }).map(deref).collect();
        trace!(?to_add, "filtered triggers by rendezvous hash");

        change_tx.send(TriggerChange::Add(to_add)).await?;
    }

    unreachable!("trigger update channel closed!")
}

async fn get_all_triggers(db: &PgPool) -> Result<HashSet<Uuid>> {
    let triggers = sqlx::query_as("
        SELECT t.id
        FROM trigger t
        JOIN job j ON t.job_id = j.id
        WHERE NOT j.paused"
    )
    .fetch_all(db)
    .await?;

    Ok(triggers.into_iter().map(first).collect())
}
