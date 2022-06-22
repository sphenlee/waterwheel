use crate::{
    messages::SchedulerUpdate,
    server::{tokens::ProcessToken, Server},
};
use anyhow::Result;
use futures::TryStreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions},
    types::FieldTable,
};
use postage::prelude::*;
use std::sync::Arc;
use lapin::options::QueueBindOptions;
use tracing::trace;
use crate::messages::TriggerUpdate;

pub const UPDATES_EXCHANGE: &str = "waterwheel.updates";

pub async fn process_updates(server: Arc<Server>) -> Result<!> {
    let chan = server.amqp_conn.create_channel().await?;

    let mut trigger_tx = server.post_office.post_mail::<TriggerUpdate>().await?;
    let mut token_tx = server.post_office.post_mail::<ProcessToken>().await?;

    // declare queue for consuming incoming messages
    let queue = chan.queue_declare(
        "", // autogenerate
        QueueDeclareOptions {
            durable: true,
            exclusive: true, // implies auto-delete
            ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_bind(
        queue.name().as_str(),
        UPDATES_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )
    .await?;

    let mut consumer = chan
        .basic_consume(
            queue.name().as_str(),
            "server",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(delivery) = consumer.try_next().await? {
        let update: SchedulerUpdate = serde_json::from_slice(&delivery.data)?;

        trace!(?update, "received scheduler update message");

        match update {
            SchedulerUpdate::TriggerUpdate(tu) => trigger_tx.send(tu).await?,
            SchedulerUpdate::ProcessToken(pt) => token_tx.send(pt).await?,
        }

        delivery.ack(BasicAckOptions::default()).await?;

        trace!("forwarded scheduler update");
    }

    unreachable!("consumer stopped consuming")
}
