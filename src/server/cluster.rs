use crate::{
    config::Config,
    messages::TriggerUpdate,
    rendezvous,
    server::{triggers::TriggerChange, Server},
    util::{deref, first},
};
///! Scheduler Cluster.
///! Multiple schedulers can form a cluster using the chitchat membership finding library
///! and then choose which triggers to manage using rendezvous hashing.
///! Membership changes are expected to be rare and don't need to be handled quickly.
use anyhow::{Context, Result};
use chitchat::{transport::UdpTransport, ChitchatConfig, ChitchatHandle, NodeId};
use chrono::Utc;
use futures::StreamExt as _;
use postage::prelude::*;
use sqlx::PgPool;
use std::{collections::HashSet, sync::Arc};
use tracing::{debug, info, trace};
use uuid::Uuid;

fn get_node_id(config: &Config) -> Result<NodeId> {
    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
    let ts = Utc::now().timestamp();
    let name = format!("{hostname}/{ts}");

    let gossip_addr = config
        .cluster_gossip_addr
        .parse()
        .context("parsing cluster_gossip_addr")?;

    Ok(NodeId::new(name, gossip_addr))
}

pub async fn start_cluster(config: &Config) -> Result<ChitchatHandle> {
    let node_id = get_node_id(config)?;
    debug!("node id is {:?}", node_id);

    let config = ChitchatConfig {
        node_id,
        cluster_id: config
            .cluster_id
            .as_deref()
            .unwrap_or("waterwheel")
            .to_owned(),
        listen_addr: config
            .cluster_gossip_bind
            .parse()
            .context("parsing cluster_gossip_bind")?,
        seed_nodes: config.cluster_seed_nodes.clone(),
        ..ChitchatConfig::default()
    };

    let chitchat_handle = chitchat::spawn_chitchat(config, vec![], &UdpTransport).await?;

    Ok(chitchat_handle)
}

pub async fn watch_live_nodes(server: Arc<Server>) -> Result<!> {
    let chitchat = server.get_chitchat().await;

    let me = chitchat.lock().await.self_node_id().id.clone();
    let mut watcher = chitchat.lock().await.live_nodes_watcher();

    let mut change_tx = server.post_office.post_mail::<TriggerChange>().await?;
    let mut current_triggers = HashSet::new();

    while let Some(live_nodes) = watcher.next().await {
        info!("cluster membership changed");
        let triggers = get_all_triggers(&server.db_pool).await?;

        let mut rendezvous = rendezvous::Rendezvous::with_me(me.clone());

        for item in live_nodes {
            rendezvous.add_node(item.id);
        }

        let mut new_triggers = HashSet::new();
        for trigger in triggers {
            if rendezvous.item_is_mine(&me, &trigger) {
                new_triggers.insert(trigger);
            }
        }

        let to_remove: Vec<_> = current_triggers
            .difference(&new_triggers)
            .map(deref)
            .collect();
        trace!("removing triggers: {:?}", to_remove);
        info!("removing {} triggers", to_remove.len());
        change_tx.send(TriggerChange::Remove(to_remove)).await?;

        let to_add: Vec<_> = new_triggers
            .difference(&current_triggers)
            .map(deref)
            .collect();
        trace!("adding triggers: {:?}", to_add);
        info!("adding {} triggers", to_add.len());
        change_tx.send(TriggerChange::Add(to_add)).await?;

        current_triggers = new_triggers;
    }

    unreachable!("chitchat watcher was closed!");
}

pub async fn trigger_update(server: Arc<Server>, update: TriggerUpdate) -> Result<()> {
    let mut change_tx = server.post_office.post_mail::<TriggerChange>().await?;

    let TriggerUpdate(uuids) = update;
    trace!(?uuids, "got trigger update");

    let chitchat = server.get_chitchat().await;
    let chitchat = chitchat.lock().await;

    let me = chitchat.self_node_id().id.clone();
    let mut rendezvous = rendezvous::Rendezvous::with_me(me.clone());

    for item in chitchat.live_nodes() {
        rendezvous.add_node(item.id.clone());
    }

    let to_add: Vec<_> = uuids
        .iter()
        .filter(|trigger| rendezvous.item_is_mine(&me, trigger))
        .map(deref)
        .collect();
    trace!(?to_add, "filtered triggers by rendezvous hash");

    if !to_add.is_empty() {
        change_tx.send(TriggerChange::Add(to_add)).await?;
    }

    Ok(())
}

async fn get_all_triggers(db: &PgPool) -> Result<HashSet<Uuid>> {
    let triggers = sqlx::query_as(
        "
        SELECT t.id
        FROM trigger t
        JOIN job j ON t.job_id = j.id
        WHERE NOT j.paused",
    )
    .fetch_all(db)
    .await?;

    Ok(triggers.into_iter().map(first).collect())
}
