use crate::messages::ConfigUpdate;
use anyhow::Result;
use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, ExchangeKind};

const CONFIG_EXCHANGE: &str = "waterwheel.config";

pub async fn setup(chan: &Channel) -> Result<()> {
    // declare outgoing exchange for config updates
    chan.exchange_declare(
        CONFIG_EXCHANGE,
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

pub async fn send(chan: &Channel, update: ConfigUpdate) -> Result<()> {
    chan.basic_publish(
        CONFIG_EXCHANGE,
        "",
        BasicPublishOptions::default(),
        serde_json::to_vec(&update)?,
        BasicProperties::default(),
    )
    .await?;

    Ok(())
}
