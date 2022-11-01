use crate::{
    messages::{ProcessToken, TriggerUpdate},
    server::updates::{TOKEN_UPDATES_EXCHANGE, TRIGGER_UPDATES_EXCHANGE},
};
use anyhow::Result;
use lapin::{
    options::{BasicPublishOptions, ExchangeDeclareOptions},
    types::FieldTable,
    BasicProperties, Channel, ExchangeKind,
};

pub async fn setup(chan: &Channel) -> Result<()> {
    // declare outgoing exchange and queue for trigger updates
    chan.exchange_declare(
        TRIGGER_UPDATES_EXCHANGE,
        ExchangeKind::Fanout,
        ExchangeDeclareOptions {
            durable: true,
            ..ExchangeDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    // declare outgoing exchange and queue for token updates
    chan.exchange_declare(
        TOKEN_UPDATES_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions {
            durable: true,
            ..ExchangeDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    Ok(())
}

pub async fn send_trigger_update(chan: &Channel, update: TriggerUpdate) -> Result<()> {
    chan.basic_publish(
        TRIGGER_UPDATES_EXCHANGE,
        "",
        BasicPublishOptions::default(),
        &serde_json::to_vec(&update)?,
        BasicProperties::default(),
    )
    .await?;

    Ok(())
}

pub async fn send_token_update(chan: &Channel, update: ProcessToken) -> Result<()> {
    chan.basic_publish(
        TOKEN_UPDATES_EXCHANGE,
        "",
        BasicPublishOptions::default(),
        &serde_json::to_vec(&update)?,
        BasicProperties::default(),
    )
    .await?;

    Ok(())
}
