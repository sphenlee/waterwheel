use crate::messages::SchedulerUpdate;
use anyhow::Result;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, ExchangeKind,
};

const UPDATES_EXCHANGE: &str = "waterwheel.updates";
const UPDATES_QUEUE: &str = "waterwheel.updates";

pub async fn setup(chan: &Channel) -> Result<()> {
    // declare outgoing exchange and queue for scheduler updates
    chan.exchange_declare(
        UPDATES_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions {
            durable: true,
            ..ExchangeDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_declare(
        UPDATES_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_bind(
        UPDATES_QUEUE,
        UPDATES_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )
    .await?;

    Ok(())
}

pub async fn send(chan: &Channel, update: SchedulerUpdate) -> Result<()> {
    chan.basic_publish(
        UPDATES_EXCHANGE,
        "",
        BasicPublishOptions::default(),
        &serde_json::to_vec(&update)?,
        BasicProperties::default(),
    )
    .await?;

    Ok(())
}
