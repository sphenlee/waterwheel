use anyhow::Result;
use bollard::container::{
    Config, CreateContainerOptions, LogsOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use futures::TryStreamExt;
use log::info;

pub async fn run_docker(image: String, args: Vec<String>, env: Vec<String>) -> Result<bool> {
    // TODO - return actual error messages from Docker
    let exit = async_std::task::spawn_blocking(move || -> Result<bool> {
        let mut rt = tokio::runtime::Builder::new()
            .basic_scheduler()
            .enable_all()
            .build()?;

        rt.block_on(async move {
            let docker = bollard::Docker::connect_with_local_defaults()?;

            info!("launching container: {}/{:?}/{:?}", image, args, env);

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

            info!("created container: {}", container.id);

            docker
                .start_container(&container.id, None::<StartContainerOptions<String>>)
                .await?;

            info!("started container: {}", container.id);

            let mut logs = docker.logs(
                &container.id,
                Some(LogsOptions {
                    follow: true,
                    stdout: true,
                    stderr: true,
                    ..LogsOptions::default()
                }),
            );

            while let Some(line) = logs.try_next().await? {
                info!("{}", line);
            }

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

            info!("container removed");

            Ok(exit == 0)
        })
    })
    .await?;

    Ok(exit)
}
