use highnoon::Result;
use std::{future::Future, sync::Once};
use waterwheel::config::{self, Config};
use testcontainers_modules::{
    rabbitmq::RabbitMq,
    postgres::Postgres,
    redis::{Redis, REDIS_PORT},
    testcontainers
};

const DEFAULT_LOG: &str = "warn,waterwheel=trace,highnoon=info,testcontainers=info,lapin=off";
static LOGGING_SETUP: Once = Once::new();

pub async fn with_external_services<F, Fut>(f: F) -> Result<()>
where
    F: FnOnce(Config) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let mut config: Config = config::loader(None)
        .set_default("db_url", "")?
        //.set_default("server_addr", "")?
        .set_override("log", DEFAULT_LOG)?
        .build()?
        .try_deserialize()?;

    LOGGING_SETUP.call_once(|| {
        waterwheel::logging::setup(&config).expect("failed to setup logging");
    });

    let client = testcontainers::clients::Cli::default();

    // start database
    let postgres = client.run(Postgres::default()
        .with_password("testpassword"));

    let port = postgres.get_host_port_ipv4(5432);
    config.db_url = format!("postgres://postgres:testpassword@localhost:{}/", port);

    // start AMQP
    let rabbit = client.run(RabbitMq);

    let port = rabbit.get_host_port_ipv4(5672);
    config.amqp_addr = format!("amqp://localhost:{}//", port);

    // start redis
    let redis = client.run(Redis::default());
    let port = redis.get_host_port_ipv4(REDIS_PORT);
    config.redis_url = format!("redis://localhost:{}", port);

    // other config setup
    config.server_addr = "http://127.0.0.1:0/".to_owned();
    config.no_authz = true;
    config.hmac_secret = Some("testing value for hmac".to_owned());
    config.cluster_gossip_bind = "127.0.0.1:0".to_owned();
    config.cluster_gossip_addr = "127.0.0.1:0".to_owned();

    // now run the test
    f(config).await
}
