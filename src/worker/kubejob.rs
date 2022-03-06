use crate::{
    messages::{TaskDef, TaskRequest},
    worker::{config_cache::get_project_config, engine::TaskEngineImpl, env, Worker, WORKER_ID},
};
use anyhow::Result;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::api::batch::v1::Job;
use kube::{
    api::{Api, PostParams},
    Client, Config, ResourceExt,
};
use std::convert::TryFrom;
use tracing::{trace, warn};

pub struct KubeJobEngine;

#[async_trait::async_trait]
impl TaskEngineImpl for KubeJobEngine {
    async fn run_task(
        &self,
        worker: &Worker,
        task_req: TaskRequest,
        task_def: TaskDef,
    ) -> Result<bool> {
        run_kubejob(worker, task_req, task_def).await
    }
}

pub async fn run_kubejob(
    worker: &Worker,
    task_req: TaskRequest,
    task_def: TaskDef,
) -> Result<bool> {
    trace!("loading kubernetes config");
    let kube_config = Config::infer().await?;
    trace!("kubernetes namespace {}", kube_config.default_namespace);
    let client = Client::try_from(kube_config)?;

    trace!("connecting to kubernetes...");
    let jobs: Api<Job> = Api::default_namespaced(client);

    let job = make_job(worker, task_req, task_def).await?;

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
            }
            Some(job) => {
                let status = job.status.as_ref().expect("status exists on job");
                trace!(pod_name=%name, "job modified, status is '{:?}'", status);

                if let Some(conditions) = &status.conditions {
                    let complete = conditions
                        .iter()
                        .any(|cond| (cond.status == "True" && cond.type_ == "Complete"));
                    let failed = conditions
                        .iter()
                        .any(|cond| (cond.status == "True" && cond.type_ == "Failed"));

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

const ONE_HOUR: i64 = 60 * 60 * 24;

async fn make_job(worker: &Worker, task_req: TaskRequest, task_def: TaskDef) -> Result<Job> {
    let env = env::get_env(&worker.config, &task_req, &task_def)?;
    let name = task_req.task_run_id.to_string();

    let config = get_project_config(worker, task_def.project_id).await?;
    let job_merge = config.get("kubernetes_job_merge");
    let ttl = config
        .get("kubernetes_job_ttl_seconds_after_finished")
        .and_then(|json| json.as_i64())
        .unwrap_or(ONE_HOUR);

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
            "ttlSecondsAfterFinished": ttl,
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

    if let Some(json) = job_merge {
        trace!("merging template: {:#} with patch: {:#}", job_json, json);
        json_patch::merge(&mut job_json, json);
    }

    trace!("job json: {:#}", job_json);

    let job = serde_json::from_value(job_json)?;
    Ok(job)
}
