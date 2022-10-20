use crate::{
    messages::{TaskDef, TaskRequest},
    worker::{docker::DockerEngine, kube::KubeEngine, kubejob::KubeJobEngine, Worker},
};
use anyhow::Result;
use std::str::FromStr;

#[derive(Copy, Clone, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskEngine {
    /// Null engine always returns success - disabled in release builds
    #[cfg(debug_assertions)]
    Null,
    /// Use a local docker instance (TODO - allow remote docker)
    Docker,
    /// Use a remote Kubernetes cluster (launching pods directly)
    Kubernetes,
    /// Use a remote Kubernetes cluster (uses jobs)
    KubernetesJobs,
}

impl FromStr for TaskEngine {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            #[cfg(debug_assertions)]
            "null" => Ok(TaskEngine::Null),
            "docker" => Ok(TaskEngine::Docker),
            "kubernetes" => Ok(TaskEngine::Kubernetes),
            "kubernetesjobs" => Ok(TaskEngine::KubernetesJobs),
            _ => Err(anyhow::Error::msg(
                "invalid engine, valid options: docker, kubernetes",
            )),
        }
    }
}

impl TaskEngine {
    pub fn get_impl(&self) -> Result<std::pin::Pin<Box<dyn TaskEngineImpl + Send + Sync + 'static>>> {
        Ok(match self {
            #[cfg(debug_assertions)]
            TaskEngine::Null => Box::pin(null::NullEngine),
            TaskEngine::Docker => Box::pin(DockerEngine),
            TaskEngine::Kubernetes => Box::pin(KubeEngine),
            TaskEngine::KubernetesJobs => Box::pin(KubeJobEngine),
        })
    }
}

#[async_trait::async_trait]
pub trait TaskEngineImpl {
    async fn run_task(
        &self,
        worker: &Worker,
        task_req: TaskRequest,
        task_def: TaskDef,
    ) -> Result<bool>;
}

#[cfg(debug_assertions)]
mod null {
    use crate::{
        messages::{TaskDef, TaskRequest},
        worker::{engine::TaskEngineImpl, Worker},
    };

    pub struct NullEngine;

    #[async_trait::async_trait]
    impl TaskEngineImpl for NullEngine {
        async fn run_task(
            &self,
            _worker: &Worker,
            _task_req: TaskRequest,
            _task_def: TaskDef,
        ) -> anyhow::Result<bool> {
            Ok(true)
        }
    }
}
