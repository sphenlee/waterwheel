use crate::amqp::get_amqp_channel;
use crate::config;
use crate::metrics;
use crate::messages::{TaskProgress, TaskRequest, TokenState};
use crate::worker::{config_cache, docker, kube, TaskEngine};
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
use std::sync::atomic::Ordering;
use cadence::{Counted, Gauged};

// TODO - queues should be configurable for task routing
const TASK_QUEUE: &str = "waterwheel.tasks";

const RESULT_EXCHANGE: &str = "waterwheel.results";
const RESULT_QUEUE: &str = "waterwheel.results";

pub async fn process_work() -> Result<!> {
    let chan = get_amqp_channel().await?;
    let statsd = metrics::get_client();

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

    let engine = config::get().task_engine;

    while let Some((chan, msg)) = consumer.try_next().await? {
        let task_req: TaskRequest = serde_json::from_slice(&msg.data)?;

        let task_def = config_cache::get_task_def(task_req.task_id).await?;

        let started_datetime = Utc::now();

        RUNNING_TASKS.fetch_add(1, Ordering::SeqCst);
        statsd.gauge_with_tags("tasks.running", RUNNING_TASKS.load(Ordering::SeqCst) as u64)
            .with_tag("worker_id", &WORKER_ID.to_string())
            .send();
        statsd.incr_with_tags("tasks.received").with_tag("worker_id", &WORKER_ID.to_string()).send();


        kvdebug!("received task", {
            task_id: task_req.task_id.to_string(),
            trigger_datetime: task_req.trigger_datetime.to_rfc3339(),
            started_datetime: started_datetime.to_rfc3339(),
            priority: format!("{:?}", task_req.priority),
        });

        let result = if task_def.image.is_some() {
            chan.basic_publish(
                RESULT_EXCHANGE,
                "",
                BasicPublishOptions::default(),
                task_progress_payload(&task_req, started_datetime, None, TokenState::Running)?,
                BasicProperties::default(),
            )
            .await?;

            let res = match engine {
                #[cfg(debug_assertions)]
                TaskEngine::Null => Ok(true),
                TaskEngine::Docker => docker::run_docker(task_req.clone(), task_def).await,
                TaskEngine::Kubernetes => kube::run_kube(task_req.clone(), task_def).await,
            };

            match res {
                Ok(true) => TokenState::Success,
                Ok(false) => TokenState::Failure,
                Err(err) => {
                    kverror!("failed to run task: {:#}", err, {
                        task_id: task_req.task_id.to_string(),
                        trigger_datetime: task_req.trigger_datetime.to_rfc3339(),
                    });
                    TokenState::Error
                }
            }
        } else {
            // task has no image, mark success immediately
            TokenState::Success
        };

        let finished_datetime = Utc::now();

        RUNNING_TASKS.fetch_sub(1, Ordering::SeqCst);
        TOTAL_TASKS.fetch_add(1, Ordering::SeqCst);

        statsd.gauge_with_tags("tasks.running", RUNNING_TASKS.load(Ordering::SeqCst) as u64)
            .with_tag("worker_id", &WORKER_ID.to_string())
            .send();
        statsd.incr_with_tags("tasks.total")
            .with_tag("worker_id", &WORKER_ID.to_string())
            .with_tag("result", result.as_str())
            .send();


        kvinfo!("task completed", {
            result: result.as_str(),
            task_id: task_req.task_id.to_string(),
            trigger_datetime: task_req.trigger_datetime.to_rfc3339(),
            started_datetime: started_datetime.to_rfc3339(),
        });

        chan.basic_publish(
            RESULT_EXCHANGE,
            "",
            BasicPublishOptions::default(),
            task_progress_payload(&task_req, started_datetime, Some(finished_datetime), result)?,
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
    task_req: &TaskRequest,
    started_datetime: DateTime<Utc>,
    finished_datetime: Option<DateTime<Utc>>,
    result: TokenState,
) -> Result<Vec<u8>> {
    let payload = serde_json::to_vec(&TaskProgress {
        task_run_id: task_req.task_run_id,
        task_id: task_req.task_id,
        trigger_datetime: task_req.trigger_datetime,
        started_datetime,
        finished_datetime,
        worker_id: *WORKER_ID,
        result,
        priority: task_req.priority,
    })?;

    Ok(payload)
}
