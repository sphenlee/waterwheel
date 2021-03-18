use crate::messages::SchedulerUpdate;
use crate::server::tokens::ProcessToken;
use crate::server::triggers::TriggerUpdate;
use crate::{amqp, postoffice};
use crate::metrics;
use anyhow::Result;
use futures::TryStreamExt;
use kv_log_macro::trace as kvtrace;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use postage::prelude::*;

const UPDATE_QUEUE: &str = "waterwheel.updates";

pub async fn process_updates() -> Result<!> {
    let chan = amqp::get_amqp_channel().await?;

    let mut trigger_tx = postoffice::post_mail::<TriggerUpdate>().await?;
    let mut token_tx = postoffice::post_mail::<ProcessToken>().await?;

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

        kvtrace!(
        "received scheduler update message", {
            update: log::kv::value::Value::from_debug(&update),
        });

        match update {
            SchedulerUpdate::TriggerUpdate(tu) => trigger_tx.send(tu).await?,
            SchedulerUpdate::ProcessToken(pt) => token_tx.send(pt).await?,
        }

        chan.basic_ack(msg.delivery_tag, BasicAckOptions::default())
            .await?;

        kvtrace!("forwarded scheduler update");
    }

    unreachable!("consumer stopped consuming")
}
