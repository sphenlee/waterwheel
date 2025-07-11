use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use lapin::{
    BasicProperties,
    options::{BasicConsumeOptions, BasicPublishOptions},
    types::FieldTable,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;
use uuid::Uuid;
use waterwheel::{
    messages::TaskDef,
    server::api,
    worker::{Worker, engine::TaskEngine, heartbeat, work},
};

mod common;

const NULL_UUID: Uuid = Uuid::from_u128(0);

#[tokio::main]
#[test]
pub async fn test_worker() -> highnoon::Result<()> {
    common::with_external_services(|mut config| async move {
        config.task_engine = TaskEngine::Null;

        let worker = Arc::new(Worker::new(config.clone()).await?);

        // insert a fake task def into the worker's cache
        {
            let mut cache = worker.task_def_cache.lock().await;
            cache.insert(
                NULL_UUID,
                Some(TaskDef {
                    task_id: NULL_UUID,
                    task_name: "testing task".to_string(),
                    job_id: NULL_UUID,
                    job_name: "testing job".to_string(),
                    project_id: NULL_UUID,
                    project_name: "testing project".to_string(),
                    image: None,
                    args: vec![],
                    env: None,
                    paused: false,
                    timeout: None,
                }),
            );
        }

        let amqp_chan = worker.amqp_conn.create_channel().await?;

        work::setup_queues(&amqp_chan, &config).await?;

        tokio::spawn(work::process_work(worker.clone()));

        // PUBLISH A TASK
        let payload = serde_json::to_vec(&json!({
            "task_run_id": NULL_UUID,
            "task_id": NULL_UUID,
            "trigger_datetime": "2000-01-01T00:00:00Z",
        }))?;

        amqp_chan
            .basic_publish(
                "",
                "waterwheel.tasks",
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?;

        //tokio::time::sleep(Duration::from_secs(5)).await;

        // WAIT FOR TASK STARTED
        let mut consumer = amqp_chan
            .basic_consume(
                "waterwheel.results",
                "test",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;
        let delivery = consumer
            .try_next()
            .await?
            .expect("no task result published");

        let mut data: Value = serde_json::from_slice(&delivery.data)?;

        let started: DateTime<Utc> = data["started_datetime"]
            .as_str()
            .expect("missing started_datetime")
            .parse()?;

        let worker_id: Uuid = data["worker_id"]
            .as_str()
            .expect("missing worker_id")
            .parse()?;

        data["started_datetime"] = "<removed>".into();
        data["worker_id"] = "<removed>".into();

        assert_eq!(
            data,
            json!({
                    "task_run_id": "00000000-0000-0000-0000-000000000000",
                    "task_id": "00000000-0000-0000-0000-000000000000",
                    "trigger_datetime": "2000-01-01T00:00:00Z",
                    "started_datetime": "<removed>",
                    "finished_datetime": null,
                    "result": "running",
                    "worker_id": "<removed>",
            })
        );

        // WAIT FOR TASK SUCCESS
        let delivery = consumer
            .try_next()
            .await?
            .expect("no task result published");

        let mut data: Value = serde_json::from_slice(&delivery.data)?;

        let started2: DateTime<Utc> = data["started_datetime"]
            .as_str()
            .expect("missing started_datetime")
            .parse()?;
        let finished: DateTime<Utc> = data["finished_datetime"]
            .as_str()
            .expect("missing finished_datetime")
            .parse()?;

        assert_eq!(started, started2);
        assert!(started2 < finished);

        let worker_id2: Uuid = data["worker_id"]
            .as_str()
            .expect("missing worker_id")
            .parse()?;

        assert_eq!(worker_id, worker_id2);

        data["started_datetime"] = "<removed>".into();
        data["finished_datetime"] = "<removed>".into();
        data["worker_id"] = "<removed>".into();

        assert_eq!(
            data,
            json!({
                    "task_run_id": "00000000-0000-0000-0000-000000000000",
                    "task_id": "00000000-0000-0000-0000-000000000000",
                    "trigger_datetime": "2000-01-01T00:00:00Z",
                    "started_datetime": "<removed>",
                    "finished_datetime": "<removed>",
                    "result": "success",
                    "worker_id": "<removed>",
            })
        );

        Ok(())
    })
    .await
}

#[tokio::main]
#[test]
pub async fn test_worker_missing_taskid() -> highnoon::Result<()> {
    common::with_external_services(|mut config| async move {
        config.task_engine = TaskEngine::Null;

        tokio::spawn(api::serve(config.clone()));
        heartbeat::wait_for_server(&config).await;

        let worker = Arc::new(Worker::new(config.clone()).await?);
        let amqp_chan = worker.amqp_conn.create_channel().await?;
        work::setup_queues(&amqp_chan, &config).await?;
        tokio::spawn(work::process_work(worker.clone()));

        // PUBLISH A TASK (no task_def in the cache!)
        let payload = serde_json::to_vec(&json!({
            "task_run_id": NULL_UUID,
            "task_id": NULL_UUID,
            "trigger_datetime": "2000-01-01T00:00:00Z",
            "priority": "normal",
        }))?;

        amqp_chan
            .basic_publish(
                "",
                "waterwheel.tasks",
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await?;

        // WAIT FOR TASK PROGRESS
        let mut consumer = amqp_chan
            .basic_consume(
                "waterwheel.results",
                "test",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        let delivery1 = timeout(Duration::from_secs(30), consumer.try_next())
            .await??
            .expect("no task result published");

        let data: Value = serde_json::from_slice(&delivery1.data)?;
        assert_eq!(data["result"].as_str(), Some("running"));

        let delivery2 = timeout(Duration::from_secs(30), consumer.try_next())
            .await??
            .expect("no task result published");

        let data: Value = serde_json::from_slice(&delivery2.data)?;
        assert_eq!(data["result"].as_str(), Some("error"));

        Ok(())
    })
    .await
}
