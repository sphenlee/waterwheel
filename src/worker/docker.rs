use crate::{
    messages::{TaskDef, TaskRequest},
    worker::{engine::TaskEngineImpl, env, Worker},
};
use anyhow::Result;
use bollard::{
    container::{
        Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
        WaitContainerOptions,
    },
    image::{CreateImageOptions, ListImagesOptions},
};
use futures::TryStreamExt;
use std::collections::HashMap;
use bollard::container::LogsOptions;
use redis::AsyncCommands;
use redis::streams::StreamMaxlen;
use tracing::trace;

pub struct DockerEngine;

#[async_trait::async_trait]
impl TaskEngineImpl for DockerEngine {
    async fn run_task(
        &self,
        worker: &Worker,
        task_req: TaskRequest,
        task_def: TaskDef,
    ) -> Result<bool> {
        run_docker(worker, task_req, task_def).await
    }
}

async fn run_docker(worker: &Worker, task_req: TaskRequest, task_def: TaskDef) -> Result<bool> {
    let docker = bollard::Docker::connect_with_local_defaults()?;

    let env = env::get_env_string(worker, &task_req, &task_def)?;

    // task_def is partially move from here down
    let image = task_def.image.unwrap();
    let args = task_def.args;

    // ____________________________________________________
    // search for the image locally
    let mut filters = HashMap::new();
    filters.insert("reference", vec![&*image]);

    trace!(?filters, "listing images");

    let list = docker
        .list_images(Some(ListImagesOptions {
            filters,
            ..ListImagesOptions::default()
        }))
        .await?;

    trace!("got {} images", list.len());

    // ____________________________________________________
    // pull the image if we didn't find it
    if list.is_empty() {
        let mut pull = docker.create_image(
            Some(CreateImageOptions::<&str> {
                from_image: &image,
                ..CreateImageOptions::default()
            }),
            None,
            None,
        );

        while let Some(data) = pull.try_next().await? {
            trace!("pulling image: {}", serde_json::to_string(&data)?);
        }
    }

    // ____________________________________________________
    // launch the container
    trace!(?image, ?args, ?env, "launching container");

    let container = docker
        .create_container(
            None::<CreateContainerOptions<String>>,
            Config {
                env: Some(env),
                cmd: Some(args),
                image: Some(image),
                ..Config::default()
            },
        )
        .await?;

    trace!(id=?container.id, "created container");

    // ____________________________________________________
    // start the container
    docker
        .start_container(&container.id, None::<StartContainerOptions<String>>)
        .await?;

    trace!(id=?container.id, "started container");

    // ____________________________________________________
    // streams the logs back
    let mut logs = docker.logs(
        &container.id,
        Some(LogsOptions::<&str> {
            follow: true,
            stdout: true,
            stderr: true,
            ..LogsOptions::default()
        }),
    );

    let key = format!("waterwheel-logs.{}", task_req.task_run_id);
    let redis_client = redis::Client::open("redis://localhost")?;
    let mut redis = redis_client.get_tokio_connection().await?;

    trace!("sending docker logs to {}", key);
    while let Some(line) = logs.try_next().await? {
        let bytes = line.into_bytes();
        trace!("got log line ({} bytes)", bytes.len());
        redis.xadd_maxlen(&key, StreamMaxlen::Approx(1024),
            "*",
            &[
                ("data", bytes.as_ref()),
            ]
        ).await?;
        trace!("sent to redis");
    }

    let _: redis::Value = redis.expire(&key, worker.config.log_retention_secs).await?;
    drop(redis);

    // ____________________________________________________
    // wait for it to terminate
    let mut waiter = docker.wait_container(&container.id, None::<WaitContainerOptions<String>>);

    let mut exit = 0;
    while let Some(x) = waiter.try_next().await? {
        trace!(id=?container.id, "container exit code: {}", x.status_code);
        exit = x.status_code;
    }

    // ____________________________________________________
    // remove the container
    docker
        .remove_container(&container.id, None::<RemoveContainerOptions>)
        .await?;

    trace!(id=?container.id, "container removed");

    Ok(exit == 0)
}
