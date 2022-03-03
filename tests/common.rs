use std::future::Future;
use testcontainers::Docker;

// TODO - remove this when testcontainers does a new release
pub mod rabbitmq {
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
}

pub async fn with_external_services<F, Fut, R>(f: F) -> R
where F: FnOnce() -> Fut,
    Fut: Future<Output=R>
{
    dotenv::dotenv().ok();

    let filter = std::env::var_os("WATERWHEEL_LOG")
        .unwrap_or_default();
    let filter = filter
        .to_str()
        .expect("WATERWHEEL_LOG env var is not UTF-8");

    waterwheel::logging::setup_raw(false, filter).expect("failed to setup logging");

    let client = testcontainers::clients::Cli::default();

    // start database
    let postgres = client.run(testcontainers::images::postgres::Postgres::default());

    let port = postgres.get_host_port(5432).expect("postgres port not exposed");
    let db_url = format!("postgres://postgres@localhost:{}/", port);
    std::env::set_var("WATERWHEEL_DB_URL", db_url);

    // start AMQP
    let rabbit = client.run(rabbitmq::RabbitMq);

    let port = rabbit.get_host_port(5672).expect("rabbitmq port not exposed");
    let amqp_addr = format!("amqp://localhost:{}//", port);
    std::env::set_var("WATERWHEEL_AMQP_ADDR", amqp_addr);

    // now run the test
    f().await
}
