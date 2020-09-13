use crate::amqp::get_amqp_channel;
use crate::messages::{TaskDef, TaskResult};
use crate::worker::docker;
use anyhow::Result;

use futures::TryStreamExt;
use kv_log_macro::{debug, error, info};
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, BasicQosOptions,
    ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};

use super::WORKER_ID;
use chrono::Utc;

// TODO - queues should be configurable for task routing
const TASK_QUEUE: &str = "waterwheel.tasks";

const RESULT_EXCHANGE: &str = "waterwheel.results";
const RESULT_QUEUE: &str = "waterwheel.results";

pub async fn process_work() -> Result<!> {
    let chan = get_amqp_channel().await?;

    // declare queue for consuming incoming messages
    let mut args = FieldTable::default();
    args.insert("x-max-priority".into(), 3.into());

    chan.queue_declare(
        TASK_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        args,
    )
    .await?;

    // declare outgoing exchange and queue for progress reports
    chan.exchange_declare(
        RESULT_EXCHANGE,
        ExchangeKind::Direct,
        ExchangeDeclareOptions {
            durable: true,
            ..ExchangeDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_declare(
        RESULT_QUEUE,
        QueueDeclareOptions {
            durable: true,
            ..QueueDeclareOptions::default()
        },
        FieldTable::default(),
    )
    .await?;

    chan.queue_bind(
        RESULT_QUEUE,
        RESULT_EXCHANGE,
        "",
        QueueBindOptions::default(),
        FieldTable::default(),
    )
    .await?;

    chan.basic_qos(1, BasicQosOptions::default()).await?;

    let mut consumer = chan
        .basic_consume(
            TASK_QUEUE,
            "worker",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    while let Some((chan, msg)) = consumer.try_next().await? {
        let task_def: TaskDef = serde_json::from_slice(&msg.data)?;

        let started_datetime = Utc::now();

        info!("received task", {
            task_id: task_def.task_id.to_string(),
            trigger_datetime: task_def.trigger_datetime.to_rfc3339(),
            started_datetime: started_datetime.to_rfc3339(),
        });

        let success = if task_def.image.is_some() {
            match docker::run_docker(task_def.clone()).await {
                Ok(_) => true,
                Err(err) => {
                    error!("failed to run task: {}", err, {
                        task_id: task_def.task_id.to_string(),
                        trigger_datetime: task_def.trigger_datetime.to_rfc3339(),
                    });
                    false
                }
            }
        } else {
            // task has no image, mark success immediately
            true
        };

        let finished_datetime = Utc::now();

        let result = match success {
            true => "success".to_string(),
            false => "failure".to_string(),
        };

        info!("task completed", {
            result: result,
            task_id: task_def.task_id.to_string(),
            trigger_datetime: task_def.trigger_datetime.to_rfc3339(),
            started_datetime: started_datetime.to_rfc3339(),
        });

        let payload = serde_json::to_vec(&TaskResult {
            task_run_id: task_def.task_run_id,
            task_id: task_def.task_id,
            trigger_datetime: task_def.trigger_datetime,
            started_datetime,
            finished_datetime,
            worker_id: *WORKER_ID,
            result,
        })?;

        chan.basic_publish(
            RESULT_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            payload,
            BasicProperties::default(),
        )
        .await?;

        chan.basic_ack(msg.delivery_tag, BasicAckOptions::default())
            .await?;
        debug!("task acked");
    }

    unreachable!("consumer stopped consuming")
}
