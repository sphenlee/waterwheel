use crate::amqp::get_amqp_channel;
use crate::messages::TaskDef;
use crate::server::tokens::Token;
use crate::{db, postoffice};
use anyhow::Result;
use lapin::options::{
    BasicPublishOptions, ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};
use log::{debug, info, warn};

const TASK_EXCHANGE: &str = "waterwheel.tasks";
const TASK_QUEUE: &str = "waterwheel.tasks";

#[derive(Debug)]
pub struct ExecuteToken(pub Token);

#[derive(sqlx::FromRow)]
struct DockerParams {
    image: Option<String>,
    args: Option<Vec<String>>,
    env: Option<Vec<String>>,
}

pub async fn process_executions() -> Result<!> {
    let pool = db::get_pool();

    let execute_rx = postoffice::receive_mail::<ExecuteToken>().await?;

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
        let token = execute_rx.recv().await?.0;
        info!("enqueueing {}", token);

        let docker: DockerParams = sqlx::query_as(
            "SELECT
                image,
                args,
                env
            FROM task
            WHERE id = $1",
        )
        .bind(&token.task_id)
        .fetch_one(&pool)
        .await?;

        if docker.image.is_none() {
            // this task does not execute anything so we can call it success right now
            // TODO
            warn!("task executes nothing - this is broken right now!")
        } else {
            let task_def = TaskDef {
                task_id: token.task_id.to_string(),
                trigger_datetime: token.trigger_datetime.to_rfc3339(),
                image: docker.image.unwrap(), // already checked none
                args: docker.args.unwrap_or_default(),
                env: docker.env,
            };

            chan.basic_publish(
                TASK_EXCHANGE,
                "",
                BasicPublishOptions::default(),
                serde_json::to_vec(&task_def)?,
                BasicProperties::default(),
            )
            .await?;
        }

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
