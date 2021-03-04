use crate::amqp::get_amqp_channel;
use crate::config;
use crate::messages::{TaskDef, TaskProgress, TokenState};
use crate::server::stash;
use crate::worker::{docker, kube};
use anyhow::Result;

use futures::TryStreamExt;
use kv_log_macro::{debug as kvdebug, error as kverror, info as kvinfo};
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicPublishOptions, BasicQosOptions,
    ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ExchangeKind};

use super::{RUNNING_TASKS, TOTAL_TASKS, WORKER_ID};
use chrono::{DateTime, Utc};
use std::str::FromStr;
use std::sync::atomic::Ordering;

enum TaskEngine {
    /// Null engine always returns success - disabled in release builds
    #[cfg(debug_assertions)]
    Null,
    /// Use a local docker instance (TODO - allow remote docker)
    Docker,
    /// Use a remote Kubernetes cluster
    Kubernetes,
}

impl FromStr for TaskEngine {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(debug_assertions)]
            "null" => Ok(TaskEngine::Null),
            "docker" => Ok(TaskEngine::Docker),
            "kubernetes" => Ok(TaskEngine::Kubernetes),
            _ => Err(anyhow::Error::msg(
                "invalid engine, valid options: docker, kubernetes",
            )),
        }
    }
}

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

    let engine: TaskEngine = config::get_or("WATERWHEEL_TASK_ENGINE", TaskEngine::Docker);

    while let Some((chan, msg)) = consumer.try_next().await? {
        let task_def: TaskDef = serde_json::from_slice(&msg.data)?;

        let started_datetime = Utc::now();

        RUNNING_TASKS.fetch_add(1, Ordering::Relaxed);

        kvinfo!("received task", {
            task_id: task_def.task_id.to_string(),
            trigger_datetime: task_def.trigger_datetime.to_rfc3339(),
            started_datetime: started_datetime.to_rfc3339(),
            priority: format!("{:?}", task_def.priority),
        });

        chan.basic_publish(
            RESULT_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            task_progress_payload(&task_def, started_datetime, None, TokenState::Running)?,
            BasicProperties::default(),
        )
        .await?;

        let result = if task_def.image.is_some() {
            let stash_jwt = stash::generate_jwt(&task_def.task_id.to_string())?;

            let res = match engine {
                #[cfg(debug_assertions)]
                TaskEngine::Null => Ok(true),
                TaskEngine::Docker => docker::run_docker(task_def.clone(), stash_jwt).await,
                TaskEngine::Kubernetes => kube::run_kube(task_def.clone(), stash_jwt).await,
            };

            match res {
                Ok(true) => TokenState::Success,
                Ok(false) => TokenState::Failure,
                Err(err) => {
                    kverror!("failed to run task: {}", err, {
                        task_id: task_def.task_id.to_string(),
                        trigger_datetime: task_def.trigger_datetime.to_rfc3339(),
                    });
                    TokenState::Error
                }
            }
        } else {
            // task has no image, mark success immediately
            TokenState::Success
        };

        let finished_datetime = Utc::now();

        RUNNING_TASKS.fetch_sub(1, Ordering::Relaxed);
        TOTAL_TASKS.fetch_add(1, Ordering::Relaxed);

        kvinfo!("task completed", {
            result: result.to_string(),
            task_id: task_def.task_id.to_string(),
            trigger_datetime: task_def.trigger_datetime.to_rfc3339(),
            started_datetime: started_datetime.to_rfc3339(),
        });

        chan.basic_publish(
            RESULT_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            task_progress_payload(&task_def, started_datetime, Some(finished_datetime), result)?,
            BasicProperties::default(),
        )
        .await?;

        chan.basic_ack(msg.delivery_tag, BasicAckOptions::default())
            .await?;
        kvdebug!("task acked");
    }

    unreachable!("consumer stopped consuming")
}

fn task_progress_payload(
    task_def: &TaskDef,
    started_datetime: DateTime<Utc>,
    finished_datetime: Option<DateTime<Utc>>,
    result: TokenState,
) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(&TaskProgress {
        task_run_id: task_def.task_run_id,
        task_id: task_def.task_id,
        trigger_datetime: task_def.trigger_datetime,
        started_datetime,
        finished_datetime,
        worker_id: *WORKER_ID,
        result,
        priority: task_def.priority,
    })?;

    Ok(payload)
}
