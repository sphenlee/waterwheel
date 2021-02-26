use crate::messages::TaskDef;
use crate::worker::env;
use anyhow::Result;
use bollard::container::{Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions, StartContainerOptions, WaitContainerOptions};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use futures::TryStreamExt;
use kv_log_macro::{info as kvinfo, trace as kvtrace};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct LogMeta<'a> {
    project_id: &'a str,
    job_id: &'a str,
    task_id: &'a str,
    trigger_datetime: &'a str,
}

#[derive(Serialize)]
struct LogMessage<'a> {
    meta: &'a LogMeta<'a>,
    msg: &'a str,
}

pub async fn run_docker(task_def: TaskDef, stash_jwt: String) -> Result<bool> {
    let docker = bollard::Docker::connect_with_local_defaults()?;

    let env = env::get_env_string(&task_def, stash_jwt)?;

    // task_def is partially move from here down
    let image = task_def.image.unwrap();
    let args = task_def.args;

    // ____________________________________________________
    // search for the image locally
    let mut filters = HashMap::new();
    filters.insert("reference", vec![&*image]);

    kvtrace!("listing images", { filters: format!("{:?}", filters) });

    let list = docker
        .list_images(Some(ListImagesOptions {
            filters,
            ..ListImagesOptions::default()
        }))
        .await?;

    kvtrace!("got {} images", list.len());

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
            kvtrace!("pulling image: {}", serde_json::to_string(&data)?);
        }
    }

    // ____________________________________________________
    // launch the container
    kvtrace!("launching container", {
        image: &image,
        args: format!("{:?}", args),
        env: format!("{:?}", env),
    });

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

    kvtrace!("created container", { id: container.id });

    // ____________________________________________________
    // start the container
    docker
        .start_container(&container.id, None::<StartContainerOptions<String>>)
        .await?;

    kvtrace!("started container", { id: container.id});

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

    let log_meta = LogMeta {
        project_id: &task_def.project_id.to_string(),
        job_id: &task_def.job_id.to_string(),
        task_id: &task_def.task_id.to_string(),
        trigger_datetime: &task_def.trigger_datetime.to_rfc3339(),
    };

    while let Some(line) = logs.try_next().await? {
        // TODO - direct these to a different log stream
        kvinfo!("{}", line, {
            project_id: log_meta.project_id,
            job_id: log_meta.job_id,
            task_id: log_meta.task_id,
            trigger_datetime: log_meta.trigger_datetime,
        });
    }

    // ____________________________________________________
    // wait for it to terminate
    let mut waiter = docker.wait_container(&container.id, None::<WaitContainerOptions<String>>);

    let mut exit = 0;
    while let Some(x) = waiter.try_next().await? {
        kvtrace!("container exit code: {}", x.status_code, { id: container.id});
        exit = x.status_code;
    }

    // ____________________________________________________
    // remove the container
    docker
        .remove_container(&container.id, None::<RemoveContainerOptions>)
        .await?;

    kvtrace!("container removed", { id: container.id });

    Ok(exit == 0)
}
