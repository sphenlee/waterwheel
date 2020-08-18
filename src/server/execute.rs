use crate::amqp::get_amqp_channel;
use crate::db;
use crate::messages::TaskDef;
use crate::server::tokens::Token;
use anyhow::Result;
use async_std::sync::Receiver;
use lapin::options::{
    BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};
use log::{debug, info};

const TASK_EXCHANGE: &str = "waterwheel.tasks";
const TASK_QUEUE: &str = "waterwheel.tasks";

pub async fn process_executions(execute_rx: Receiver<Token>) -> Result<!> {
    let pool = db::get_pool();

    let chan = get_amqp_channel().await?;

    chan.exchange_declare(
        TASK_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions {
            durable: true,
            ..ExchangeDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_declare(
        TASK_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_bind(
        TASK_QUEUE,
        TASK_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )
    .await?;

    // TODO - recover any tasks

    loop {
        let token = execute_rx.recv().await?;
        info!("enqueueing {}", token);

        let mut task_def = sqlx::query_as::<_, TaskDef>(
            "SELECT
                CAST(id AS VARCHAR) AS task_id,
                '' AS trigger_datetime,
                image,
                args,
                env
            FROM task
            WHERE id = $1",
        )
        .bind(&token.task_id)
        .fetch_one(&pool)
        .await?;

        task_def.trigger_datetime = token.trigger_datetime.to_rfc3339();

        chan.basic_publish(
            TASK_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            serde_json::to_vec(&task_def)?,
            BasicProperties::default(),
        )
        .await?;

        sqlx::query(
            "UPDATE token
            SET state = 'active',
                count = count - (SELECT threshold FROM task WHERE id = task_id)
            WHERE task_id = $1
            AND trigger_datetime = $2",
        )
        .bind(token.task_id)
        .bind(token.trigger_datetime)
        .execute(&pool)
        .await?;

        debug!("done enqueueing {}", token);
    }
}
