use highnoon::Result;
use std::future::Future;
use testcontainers::Docker;
use waterwheel::config::{self, Config};

pub mod rabbitmq;

const DEFAULT_LOG: &str = "warn,waterwheel=trace,highnoon=info,testcontainers=info,lapin=off";

pub async fn with_external_services<F, Fut>(f: F) -> Result<()>
where F: FnOnce(Config) -> Fut,
    Fut: Future<Output=Result<()>>
{
    let mut config: Config = config::loader()
        .set_default("db_url", "")?
        .set_default("server_addr", "")?
        .set_override("log", DEFAULT_LOG)?
        .build()?
        .try_deserialize()?;

    waterwheel::logging::setup(&config)?;

    let client = testcontainers::clients::Cli::default();

    // start database
    let postgres = client.run(testcontainers::images::postgres::Postgres::default());

    let port = postgres.get_host_port(5432).expect("postgres port not exposed");
    config.db_url = format!("postgres://postgres@localhost:{}/", port);

    // start AMQP
    let rabbit = client.run(rabbitmq::RabbitMq);

    let port = rabbit.get_host_port(5672).expect("rabbitmq port not exposed");
    config.amqp_addr = format!("amqp://localhost:{}//", port);

    // other config setup
    config.server_addr = "http://127.0.0.1:8080/".to_owned();
    config.no_authz = true;
    config.hmac_secret = Some("testing value for hmac".to_owned());

    // now run the test
    f(config).await
}

