use crate::messages::{TaskDef, TaskRequest};
use crate::worker::env;
use anyhow::Result;
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use futures::TryStreamExt;
use std::collections::HashMap;
use tracing::{trace};
use crate::worker::engine::TaskEngineImpl;

pub struct DockerEngine;

#[async_trait::async_trait]
impl TaskEngineImpl for DockerEngine {
    async fn run_task(&self, task_req: TaskRequest, task_def: TaskDef) -> Result<bool> {
        run_docker(task_req, task_def).await
    }
}

async fn run_docker(task_req: TaskRequest, task_def: TaskDef) -> Result<bool> {
    let docker = bollard::Docker::connect_with_local_defaults()?;

    let env = env::get_env_string(&task_req, &task_def)?;

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

    // // ____________________________________________________
    // // streams the logs back
    // let mut logs = docker.logs(
    //     &container.id,
    //     Some(LogsOptions::<&str> {
    //         follow: true,
    //         stdout: true,
    //         stderr: true,
    //         ..LogsOptions::default()
    //     }),
    // );
    //
    // let log_meta = LogMeta {
    //     project_id: &task_def.project_id.to_string(),
    //     job_id: &task_def.job_id.to_string(),
    //     task_id: &task_def.task_id.to_string(),
    //     trigger_datetime: &task_req.trigger_datetime.to_rfc3339(),
    // };
    //
    // while let Some(line) = logs.try_next().await? {
    //     info!(target: "container_logs",
    //         project_id=?log_meta.project_id,
    //         job_id=?log_meta.job_id,
    //         task_id=?log_meta.task_id,
    //         trigger_datetime=?log_meta.trigger_datetime,
    //         "{}", line);
    // }

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
