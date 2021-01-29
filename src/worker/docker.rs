use crate::messages::TaskDef;
use anyhow::Result;
use bollard::container::{
    Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use futures::TryStreamExt;
use kv_log_macro::{info, trace};
use serde::Serialize;
use std::collections::HashMap;
use tokio::io::AsyncWriteExt;

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

pub async fn run_docker(task_def: TaskDef) -> Result<bool> {
    // TODO - return actual error messages from Docker
    let image = task_def.image.unwrap();
    let args = task_def.args;
    let mut env = task_def.env.unwrap_or_default();

    env.push(format!(
        "WATERWHEEL_TRIGGER_DATETIME={}",
        task_def
            .trigger_datetime
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    ));
    env.push(format!("WATERWHEEL_TASK_NAME={}", task_def.task_name));
    env.push(format!("WATERWHEEL_TASK_ID={}", task_def.task_id));
    env.push(format!("WATERWHEEL_JOB_NAME={}", task_def.job_name));
    env.push(format!("WATERWHEEL_JOB_ID={}", task_def.job_id));
    env.push(format!("WATERWHEEL_PROJECT_NAME={}", task_def.project_name));
    env.push(format!("WATERWHEEL_PROJECT_ID={}", task_def.project_id));

    let docker = bollard::Docker::connect_with_local_defaults()?;

    let mut filters = HashMap::new();
    filters.insert("reference", vec![&*image]);

            trace!("listing images", { filters: format!("{:?}", filters) });

            let list = docker
                .list_images(Some(ListImagesOptions {
        filters,
        ..ListImagesOptions::default()
                }))
                .await?;

    trace!("got {} images", list.len());

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

    info!("launching container", {
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

    trace!("created container", { id: container.id });

    docker
        .start_container(&container.id, None::<StartContainerOptions<String>>)
        .await?;

    trace!("started container", { id: container.id});

    let mut logs = docker.logs(
        &container.id,
        Some(LogsOptions::<&str> {
            follow: true,
            stdout: true,
            stderr: true,
            ..LogsOptions::default()
        }),
    );

    let vector_addr = std::env::var("WATERWHEEL_VECTOR_ADDR")?;
    let mut vector = tokio::net::TcpStream::connect(&vector_addr).await?;

    let log_meta = LogMeta {
        project_id: &task_def.project_id.to_string(),
        job_id: &task_def.job_id.to_string(),
        task_id: &task_def.task_id.to_string(),
        trigger_datetime: &task_def.trigger_datetime.to_rfc3339(),
    };

    while let Some(line) = logs.try_next().await? {
        vector
            .write(&serde_json::to_vec(&LogMessage {
                meta: &log_meta,
                msg: &format!("{}", line),
            })?)
            .await?;
        vector.write(b"\n").await?;
    }

    vector.shutdown().await?;

    let mut waiter =
        docker.wait_container(&container.id, None::<WaitContainerOptions<String>>);

    let mut exit = 0;
    while let Some(x) = waiter.try_next().await? {
        info!("container exit code: {}", x.status_code);
        exit = x.status_code;
    }

    docker
        .remove_container(&container.id, None::<RemoveContainerOptions>)
        .await?;

    trace!("container removed", { id: container.id });

    Ok(exit == 0)
}
