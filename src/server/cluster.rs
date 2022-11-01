use crate::{
    config::Config,
    messages::TriggerUpdate,
    server::{triggers::TriggerChange, Server},
    util::{deref, first, spawn_or_crash},
};
///! Scheduler Cluster.
///! Multiple schedulers can form a cluster using the chitchat membership finding library
///! and then choose which triggers to manage using rendezvous hashing.
///! Membership changes are expected to be rare and don't need to be handled quickly.
use anyhow::{Context, Result};
use chitchat::{transport::UdpTransport, ChitchatConfig, NodeId};
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
    let name = format!("{}/{}", hostname, ts);

    let gossip_addr = config
        .cluster_gossip_addr
        .parse()
        .context("parsing cluster_gossip_addr")?;

    Ok(NodeId::new(name, gossip_addr))
}

pub async fn start_cluster(server: &mut Arc<Server>) -> Result<()> {
    let node_id = get_node_id(&server.config)?;
    debug!("node id is {:?}", node_id);

    let config = ChitchatConfig {
        node_id,
        cluster_id: server
            .config
            .cluster_id
            .as_deref()
            .unwrap_or("waterwheel")
            .to_owned(),
        listen_addr: server
            .config
            .cluster_gossip_bind
            .parse()
            .context("parsing cluster_gossip_bind")?,
        seed_nodes: server.config.cluster_seed_nodes.clone(),
        ..ChitchatConfig::default()
    };

    let chitchat_handle = chitchat::spawn_chitchat(config, vec![], &UdpTransport).await?;

    {
        let server = Arc::get_mut(server).expect("someone else has a reference to the server!");

        server.cluster = Some(chitchat_handle);
    }

    spawn_or_crash("watch_live_nodes", server.clone(), watch_live_nodes);

    Ok(())
}

async fn watch_live_nodes(server: Arc<Server>) -> Result<!> {
    let chitchat = server.get_chitchat().await;

    let me = chitchat.lock().await.self_node_id().id.clone();
    let mut watcher = chitchat.lock().await.live_nodes_watcher();

    let mut change_tx = server.post_office.post_mail::<TriggerChange>().await?;
    let mut current_triggers = HashSet::new();

    while let Some(live_nodes) = watcher.next().await {
        info!("cluster membership changed");
        let triggers = get_all_triggers(&server.db_pool).await?;

        let h =
            std::hash::BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default();
        let mut rendezvous = hash_rings::rendezvous::Client::with_hasher(h);

        rendezvous.insert_node(&me, 1);
        for item in &live_nodes {
            rendezvous.insert_node(&item.id, 1);
        }

        for trigger in &triggers {
            rendezvous.insert_point(trigger);
        }

        let new_triggers: HashSet<_> = rendezvous.get_points(&me).into_iter().map(deref).collect();

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

    let h = std::hash::BuildHasherDefault::<std::collections::hash_map::DefaultHasher>::default();
    let mut rendezvous = hash_rings::rendezvous::Client::with_hasher(h);

    let chitchat = server.get_chitchat().await;
    let chitchat = chitchat.lock().await;

    let me = &chitchat.self_node_id().id;
    rendezvous.insert_node(me, 1);

    for item in chitchat.live_nodes() {
        rendezvous.insert_node(&item.id, 1);
    }

    let to_add = uuids
        .iter()
        .filter(|trigger| rendezvous.get_node(trigger) == &chitchat.self_node_id().id)
        .map(deref)
        .collect();
    trace!(?to_add, "filtered triggers by rendezvous hash");

    change_tx.send(TriggerChange::Add(to_add)).await?;

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
