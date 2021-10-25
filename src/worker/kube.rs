use crate::messages::{TaskDef, TaskRequest};
use crate::worker::config_cache::get_project_config;
use crate::worker::env;
use crate::worker::WORKER_ID;
use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, DeleteParams, PostParams};
use kube::{Client, Config, ResourceExt};
use std::convert::TryFrom;
use tracing::{trace, warn};
use crate::worker::engine::TaskEngineImpl;

pub struct KubeEngine;

#[async_trait::async_trait]
impl TaskEngineImpl for KubeEngine {
    async fn run_task(&self, task_req: TaskRequest, task_def: TaskDef) -> Result<bool> {
        run_kube(task_req, task_def).await
    }
}


pub async fn run_kube(task_req: TaskRequest, task_def: TaskDef) -> Result<bool> {
    trace!("loading kubernetes config");
    let config = Config::infer().await?;
    trace!("kubernetes namespace {}", config.default_namespace);
    let client = Client::try_from(config)?;

    trace!("connecting to kubernetes...");
    let pods: Api<Pod> = Api::default_namespaced(client);

    let pod = make_pod(task_req, task_def).await?;

    // Create the pod
    let pod = pods.create(&PostParams::default(), &pod).await?;
    let name = pod.name();

    let mut watcher = kube_runtime::watcher::watch_object(pods.clone(), &name).boxed();

    let mut result = false;
    while let Some(maybe_pod) = watcher.try_next().await? {
        match maybe_pod {
            None => {
                warn!(pod_name=%name, "pod was deleted externally");
                anyhow::bail!("pod was deleted externally");
            },
            Some(pod) => {
                let status = pod.status.as_ref().expect("status exists on pod");
                let phase = status.phase.clone().unwrap_or_default();
                trace!(pod_name=%pod.name(), "pod modified, phase is '{}'", phase);

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

    // let mut logs = pods
    //     .log_stream(
    //         &name,
    //         &LogParams {
    //             //previous: true,
    //             follow: true,
    //             ..LogParams::default()
    //         },
    //     )
    //     .await?;
    //
    // while let Some(log) = logs.try_next().await? {
    //     // TODO - kubernetes probably doesn't need this, logs can be shipped from the cluster
    //     let line = String::from_utf8_lossy(&*log);
    //     info!(target: "container_logs",
    //         "{}", line.trim_end());
    // }

    trace!(pod_name=%name, "deleting pod");
    let _ = pods.delete(&name, &DeleteParams::default()).await?;

    Ok(result)
}

async fn make_pod(task_req: TaskRequest, task_def: TaskDef) -> Result<Pod> {
    let env = env::get_env(&task_req, &task_def)?;
    let name = task_req.task_run_id.to_string();

    // Create a pod from JSON
    let mut pod_json = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": name,
            "labels": {
                "worker_id": *WORKER_ID,
                "task_id": task_req.task_id,
                "job_id": task_def.job_id,
                "project_id": task_def.project_id,
            },
        },
        "spec": {
            "containers": [
                {
                    "name": name,
                    "image": task_def.image.unwrap(),
                    "args": task_def.args,
                    "env": env,
                },
            ],
            "restartPolicy": "Never",
        }
    });

    let config = get_project_config(task_def.project_id).await?;
    let pod_merge = config.get("kubernetes_pod_merge");

    if let Some(json) = pod_merge {
        trace!("merging template: {:#} with patch: {:#}", pod_json, json);
        json_patch::merge(&mut pod_json, json);
    }

    trace!("pod json: {:#}", pod_json);

    let pod = serde_json::from_value(pod_json)?;
    Ok(pod)
}
