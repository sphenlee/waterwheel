use crate::messages::{TaskDef, TaskRequest};
use crate::worker::config_cache::get_project_config;
use crate::worker::env;
use crate::worker::WORKER_ID;
use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use kube::api::{Api, PostParams};
use kube::{Client, Config, ResourceExt};
use std::convert::TryFrom;
use tracing::{trace, warn};
use crate::worker::engine::TaskEngineImpl;
use k8s_openapi::api::batch::v1::Job;

pub struct KubeJobEngine;

#[async_trait::async_trait]
impl TaskEngineImpl for KubeJobEngine {
    async fn run_task(&self, task_req: TaskRequest, task_def: TaskDef) -> Result<bool> {
        run_kubejob(task_req, task_def).await
    }
}


pub async fn run_kubejob(task_req: TaskRequest, task_def: TaskDef) -> Result<bool> {
    trace!("loading kubernetes config");
    let config = Config::infer().await?;
    trace!("kubernetes namespace {}", config.default_namespace);
    let client = Client::try_from(config)?;

    trace!("connecting to kubernetes...");
    let jobs: Api<Job> = Api::default_namespaced(client);

    let job = make_job(task_req, task_def).await?;

    // Create the pod
    let job = jobs.create(&PostParams::default(), &job).await?;
    let name = job.name();

    let mut watcher = kube_runtime::watcher::watch_object(jobs.clone(), &name).boxed();

    let mut result = false;
    while let Some(maybe_job) = watcher.try_next().await? {
        match maybe_job {
            None => {
                warn!(job_name=%name, "job was deleted externally");
                anyhow::bail!("job was deleted externally");
            },
            Some(job) => {
                let status = job.status.as_ref().expect("status exists on job");
                trace!(pod_name=%name, "job modified, status is '{:?}'", status);

                if let Some(conditions) = &status.conditions {
                    let complete = conditions.iter().any(|cond| (cond.status == "True" && cond.type_ == "Complete"));
                    let failed = conditions.iter().any(|cond| (cond.status == "True" && cond.type_ == "Failed"));

                    if complete {
                        result = true;
                        break;
                    }
                    if failed {
                        break;
                    }
                }
            }
        }
    }

    Ok(result)
}

async fn make_job(task_req: TaskRequest, task_def: TaskDef) -> Result<Job> {
    let env = env::get_env(&task_req, &task_def)?;
    let name = task_req.task_run_id.to_string();

    let meta = serde_json::json!({
        "name": name,
        "labels": {
            "worker_id": *WORKER_ID,
            "task_id": task_req.task_id,
            "job_id": task_def.job_id,
            "project_id": task_def.project_id,
        },
    });

    // Create a pod from JSON
    let mut job_json = serde_json::json!({
        "apiVersion": "batch/v1",
        "kind": "Job",
        "metadata": meta,
        "spec": {
            "template": {
                "metadata": meta,
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
            }
        }
    });

    let config = get_project_config(task_def.project_id).await?;
    let job_merge = config.get("kubernetes_job_merge");

    if let Some(json) = job_merge {
        trace!("merging template: {:#} with patch: {:#}", job_json, json);
        json_patch::merge(&mut job_json, json);
    }

    trace!("job json: {:#}", job_json);

    let job = serde_json::from_value(job_json)?;
    Ok(job)
}
