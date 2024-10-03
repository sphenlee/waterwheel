use highnoon::Result;
use std::{future::Future, sync::{atomic, Once}};
use waterwheel::config::{self, Config};
use testcontainers_modules::{
    postgres::Postgres, rabbitmq::RabbitMq, redis::{Redis, REDIS_PORT}, testcontainers::{self, runners::AsyncRunner}
};

const DEFAULT_LOG: &str = "warn,waterwheel=trace,highnoon=info,testcontainers=info,lapin=off";
static LOGGING_SETUP: Once = Once::new();

// hopefully we have some free ports starting from here - we increment for each test run
static AVAILABLE_PORT: atomic::AtomicU16 = atomic::AtomicU16::new(8200);

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

    // start database
    let postgres = Postgres::default().with_password("testpassword").start().await?;

    let port = postgres.get_host_port_ipv4(5432).await?;
    config.db_url = format!("postgres://postgres:testpassword@localhost:{}/", port);

    // start AMQP
    let rabbit = RabbitMq::default().start().await?;

    let port = rabbit.get_host_port_ipv4(5672).await?;
    config.amqp_addr = format!("amqp://localhost:{}//", port);

    // start redis
    let redis = Redis::default().start().await?;
    let port = redis.get_host_port_ipv4(REDIS_PORT).await?;
    config.redis_url = format!("redis://localhost:{}", port);

    // grab a (hopefully) unused port
    let port = AVAILABLE_PORT.fetch_add(1, atomic::Ordering::SeqCst);

    // other config setup
    config.server_bind = format!("127.0.0.1:{port}");
    config.server_addr = format!("http://127.0.0.1:{port}/");
    config.no_authz = true;
    config.hmac_secret = Some("testing value for hmac".to_owned());
    config.cluster_gossip_bind = "127.0.0.1:0".to_owned();
    config.cluster_gossip_addr = "127.0.0.1:0".to_owned();

    // now run the test
    f(config).await
}
