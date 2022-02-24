use crate::{
    messages::SchedulerUpdate,
    server::{tokens::ProcessToken, triggers::TriggerUpdate, Server},
};
use anyhow::Result;
use futures::TryStreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions},
    types::FieldTable,
};
use postage::prelude::*;
use std::sync::Arc;
use tracing::trace;

const UPDATE_QUEUE: &str = "waterwheel.updates";

pub async fn process_updates(server: Arc<Server>) -> Result<!> {
    let chan = server.amqp_conn.create_channel().await?;

    let mut trigger_tx = server.post_office.post_mail::<TriggerUpdate>().await?;
    let mut token_tx = server.post_office.post_mail::<ProcessToken>().await?;

    // declare queue for consuming incoming messages
    chan.queue_declare(
        UPDATE_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    let mut consumer = chan
        .basic_consume(
            UPDATE_QUEUE,
            "server",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some((chan, msg)) = consumer.try_next().await? {
        let update: SchedulerUpdate = serde_json::from_slice(&msg.data)?;

        trace!(?update, "received scheduler update message");

        match update {
            SchedulerUpdate::TriggerUpdate(tu) => trigger_tx.send(tu).await?,
            SchedulerUpdate::ProcessToken(pt) => token_tx.send(pt).await?,
        }

        chan.basic_ack(msg.delivery_tag, BasicAckOptions::default())
            .await?;

        trace!("forwarded scheduler update");
    }

    unreachable!("consumer stopped consuming")
}
