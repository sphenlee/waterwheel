use anyhow::Result;
use crate::messages::TaskDef;
use futures::{StreamExt, TryStreamExt};
use kube::api::{Api, ListParams, PostParams, WatchEvent, DeleteParams, LogParams, Meta};
use kube::Client;
use k8s_openapi::api::core::v1::Pod;
use kv_log_macro::{trace as kvtrace, warn as kvwarn, info as kvinfo, debug as kvdebug};
use crate::worker::env;
use crate::worker::WORKER_ID;

pub async fn run_kube(task_def: TaskDef, stash_jwt: String) -> Result<bool> {
    let client = Client::try_default().await?;

    let ns = std::env::var("WATERWHEEL_KUBE_NAMESPACE").unwrap_or_else(|_| "default".to_owned());
    let pods: Api<Pod> = Api::namespaced(client, &ns);

    kvtrace!("connected to Kubernetes namespace {}", ns);

    let env = env::get_env(&task_def, stash_jwt)?;

    let name = task_def.task_run_id.to_string();

    // Create a pod from JSON
    let pod_json = serde_json::json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": name,
            "labels": {
                "worker_id": *WORKER_ID,
                "task_id": task_def.task_id,
                "job_id": task_def.job_id,
                "project_id": task_def.project_id,
            },
            "annotations": {
                "atlassian.com/business_unit": "Data Platform"
            }
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
    kvtrace!("pod json: {:#}", pod_json);

    let pod = serde_json::from_value(pod_json)?;

    // Create the pod
    let _pod = pods.create(&PostParams::default(), &pod).await?;

    // Start a watch call for pods matching our name
    let lp = ListParams::default().fields(&format!("metadata.name={}", name));
    let mut stream = pods.watch(&lp, "0").await?.boxed();

    let mut result = false;
    while let Some(status) = stream.try_next().await? {
        match status {
            WatchEvent::Added(pod) => {
                kvdebug!("pod created", {
                    pod_name: Meta::name(&pod),
                });
            },
            WatchEvent::Modified(pod) => {
                let status = pod.status.as_ref().expect("status exists on pod");
                let phase = status.phase.clone().unwrap_or_default();
                kvtrace!("pod modified, phase is '{}'", phase, {
                    pod_name: Meta::name(&pod),
                });

                if phase == "Succeeded"{
                    result = true;
                    break;
                }
                if phase == "Failed" {
                    break;
                }
            }
            //WatchEvent::Deleted(o) => println!("Deleted {}", Meta::name(&o)),
            WatchEvent::Error(e) => {
                kvwarn!("error from Kubernetes {:?}", e, {
                    pod_name: name
                });
                return Err(e.into());
            },
            _ => {}
        }
    };

    let mut logs = pods.log_stream(&name, &LogParams {
        //previous: true,
        follow: true,
        ..LogParams::default()
    }).await?;

    while let Some(log) = logs.try_next().await? {
        // TODO - direct these to a different log stream
        // TODO - kubernetes probably doesn't need this, logs can be shipped from the cluster
        let line = String::from_utf8_lossy(&*log);
        kvinfo!("{}", line.trim_end());
    }

    kvtrace!("deleting pod", {pod_name: name});
    let _ = pods.delete(&name, &DeleteParams::default()).await?;

    Ok(result)
}
