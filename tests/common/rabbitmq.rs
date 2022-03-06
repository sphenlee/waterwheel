// TODO - remove this when testcontainers does a new release

use std::collections::HashMap;
use testcontainers::{Container, Docker, Image, WaitForMessage};

const NAME: &str = "rabbitmq";
const TAG: &str = "3.8.22-management";

#[derive(Debug, Default, Clone)]
pub struct RabbitMq;

impl Image for RabbitMq {
    type Args = Vec<String>;
    type EnvVars = HashMap<String, String>;
    type Volumes = HashMap<String, String>;
    type EntryPoint = std::convert::Infallible;

    fn descriptor(&self) -> String {
        format!("{}:{}", NAME, TAG)
    }

    fn wait_until_ready<D: Docker>(&self, container: &Container<'_, D, Self>) {
        container
            .logs()
            .stdout
            .wait_for_message("Server startup complete; 4 plugins started.")
            .unwrap();
    }

    fn args(&self) -> Self::Args {
        vec![]
    }

    fn env_vars(&self) -> Self::EnvVars {
        HashMap::new()
    }

    fn volumes(&self) -> Self::Volumes {
        HashMap::new()
    }

    fn with_args(self, _args: Self::Args) -> Self {
        self
    }
}
