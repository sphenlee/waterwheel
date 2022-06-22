use crate::messages::SchedulerUpdate;
use anyhow::Result;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, ExchangeKind,
};
use crate::server::updates::UPDATES_EXCHANGE;

pub async fn setup(chan: &Channel) -> Result<()> {
    // declare outgoing exchange and queue for scheduler updates
    chan.exchange_declare(
        UPDATES_EXCHANGE,
        ExchangeKind::Fanout,
        ExchangeDeclareOptions {
            durable: true,
            ..ExchangeDeclareOptions::default()
        },
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
