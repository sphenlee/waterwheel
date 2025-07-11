use crate::{
    messages::{ProcessToken, TriggerUpdate},
    server::{Server, triggers::trigger_update},
};
use anyhow::Result;
use futures::TryStreamExt;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
};
use postage::prelude::*;
use std::sync::Arc;
use tracing::trace;

pub const TRIGGER_UPDATES_EXCHANGE: &str = "waterwheel.updates.triggers";
pub const TOKEN_UPDATES_EXCHANGE: &str = "waterwheel.updates.tokens";
pub const TOKEN_UPDATES_QUEUE: &str = "waterwheel.updates.tokens";

pub async fn process_token_updates(server: Arc<Server>) -> Result<!> {
    let chan = server.amqp_conn.create_channel().await?;

    let mut token_tx = server.post_office.post_mail::<ProcessToken>().await?;

    // declare queue for consuming incoming messages
    chan.queue_declare(
        TOKEN_UPDATES_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_bind(
        TOKEN_UPDATES_QUEUE,
        TOKEN_UPDATES_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )
    .await?;

    let mut consumer = chan
        .basic_consume(
            TOKEN_UPDATES_QUEUE,
            "scheduler",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(delivery) = consumer.try_next().await? {
        let update: ProcessToken = serde_json::from_slice(&delivery.data)?;
        trace!(?update, "received token update message");

        token_tx.send(update).await?;
        delivery.ack(BasicAckOptions::default()).await?;
        trace!("forwarded token update");
    }

    unreachable!("consumer stopped consuming")
}

pub async fn process_trigger_updates(server: Arc<Server>) -> Result<!> {
    let chan = server.amqp_conn.create_channel().await?;

    // declare queue for consuming incoming messages
    let queue = chan
        .queue_declare(
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
        TRIGGER_UPDATES_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )
    .await?;

    let mut consumer = chan
        .basic_consume(
            queue.name().as_str(),
            "scheduler",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some(delivery) = consumer.try_next().await? {
        let update: TriggerUpdate = serde_json::from_slice(&delivery.data)?;
        trace!(?update, "received trigger update message");

        trigger_update(server.clone(), update).await?;

        delivery.ack(BasicAckOptions::default()).await?;
        trace!("forwarded scheduler update");
    }

    unreachable!("consumer stopped consuming")
}
