use crate::{
    messages::{TaskDef, TaskRequest},
    worker::{config_cache::get_project_config, engine::TaskEngineImpl, env, Worker, WORKER_ID},
};
use anyhow::Result;
use futures::{AsyncBufReadExt, StreamExt, TryStreamExt};
use itertools::Itertools;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, DeleteParams, LogParams, PostParams},
    Client, Config, ResourceExt,
};
use rand::seq::SliceRandom;
use redis::{streams::StreamMaxlen, AsyncCommands};
use std::{convert::TryFrom, time::Duration};
use tracing::{trace, warn};

const DELETE_POD_TIMEOUT: Duration = Duration::from_secs(5 * 60);

pub struct KubeEngine;

#[async_trait::async_trait]
impl TaskEngineImpl for KubeEngine {
    async fn run_task(
        &self,
        worker: &Worker,
        task_req: TaskRequest,
        task_def: TaskDef,
    ) -> Result<bool> {
        run_kube(worker, task_req, task_def).await
    }
}

pub async fn run_kube(worker: &Worker, task_req: TaskRequest, task_def: TaskDef) -> Result<bool> {
    trace!("loading kubernetes config");
    let kube_config = Config::infer().await?;
    trace!("kubernetes namespace {}", kube_config.default_namespace);
    let client = Client::try_from(kube_config)?;

    trace!("connecting to kubernetes...");
    let pods: Api<Pod> = Api::default_namespaced(client);

    let pod = make_pod(worker, &task_req, task_def).await?;
    let name = pod.name_any();

    // Create the pod
    trace!(pod_name=%name, "creating pod");
    let _pod = pods.create(&PostParams::default(), &pod).await?;
    trace!(pod_name=%name, "created pod");

    let mut watcher = kube_runtime::watcher::watch_object(pods.clone(), &name).boxed();

    let mut result = false;

    trace!(pod_name=%name, "watching pod");

    while let Some(maybe_pod) = watcher.try_next().await? {
        match maybe_pod {
            None => {
                warn!(pod_name=%name, "pod was deleted externally");
                anyhow::bail!("pod was deleted externally");
            }
            Some(pod) => {
                let status = pod.status.as_ref().expect("status exists on pod");
                let phase = status.phase.clone().unwrap_or_default();
                trace!(pod_name=%pod.name_any(), "pod modified, phase is '{}'", phase);

                if phase == "Succeeded" {
                    result = true;
                    break;
                }
                if phase == "Failed" {
                    break;
                }
            }
        }
    }

    let mut logs = pods
        .log_stream(
            &name,
            &LogParams {
                follow: true,
                ..LogParams::default()
            },
        )
        .await?
        .lines();

    let key = format!("waterwheel-logs.{}", task_req.task_run_id);
    let mut redis = worker.redis_client.get_multiplexed_tokio_connection().await?;

    trace!("sending kubernetes pod logs to {}", key);
    while let Some(line) = logs.try_next().await? {
        trace!("got log line ({} bytes)", line.len());
        let _: () = redis
            .xadd_maxlen(
                &key,
                StreamMaxlen::Approx(1024),
                "*",
                &[("data", line)],
            )
            .await?;
        trace!("sent to redis");
    }

    let _: redis::Value = redis.expire(&key, worker.config.log_retention.try_into()?).await?;
    drop(redis);

    trace!(pod_name=%name, "deleting pod");

    match tokio::time::timeout(
        DELETE_POD_TIMEOUT,
        pods.delete(&name, &DeleteParams::default()),
    )
    .await
    {
        Ok(inner) => {
            inner?;
        }
        Err(_) => {
            warn!(pod_name=%name, "timeout while deleting pod");
        }
    }
    trace!(pod_name=%name, "deleted pod");

    Ok(result)
}

// TODO - make this a util, we should use this grist in a few other places too
fn make_grist() -> String {
    let mut rng = rand::thread_rng();
    std::iter::from_fn(move || {
        let byte = b"ghjklmnpqrstuvwxyz"
            .choose(&mut rng)
            .expect("slice is not empty");
        Some(char::from(*byte))
    })
    .take(6)
    .join("")
}

async fn make_pod(worker: &Worker, task_req: &TaskRequest, task_def: TaskDef) -> Result<Pod> {
    let env = env::get_env(worker, task_req, &task_def)?;

    let grist = make_grist();
    let name = format!("{}--{}", task_req.task_run_id, grist);

    // Create a pod from JSON
    let mut pod_json = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": name,
            "labels": {
                "worker_id": *WORKER_ID,
                "task_id": task_req.task_id,
                "task_run_id": task_req.task_run_id,
                "job_id": task_def.job_id,
                "project_id": task_def.project_id,
            },
        },
        "spec": {
            "containers": [
                {
                    "name": "task",
                    "image": task_def.image.unwrap(),
                    "args": task_def.args,
                    "env": env,
                },
            ],
            "restartPolicy": "Never",
        }
    });

    let config = get_project_config(worker, task_def.project_id).await?;
    let pod_merge = config.get("kubernetes_pod_merge");

    if let Some(json) = pod_merge {
        trace!("merging template: {:#} with patch: {:#}", pod_json, json);
        json_patch::merge(&mut pod_json, json);
    }

    trace!("pod json: {:#}", pod_json);

    let pod = serde_json::from_value(pod_json)?;
    Ok(pod)
}
