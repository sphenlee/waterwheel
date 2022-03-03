use std::future::Future;
use testcontainers::Docker;

pub mod rabbitmq;

pub async fn with_external_services<F, Fut, R>(f: F) -> R
where F: FnOnce() -> Fut,
    Fut: Future<Output=R>
{
    setup_logging();

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

    // other config setup
    std::env::set_var("WATERWHEEL_SERVER_ADDR", "http://no.such.host/");
    std::env::set_var("WATERWHEEL_NO_AUTHZ", "1");

    // now run the test
    f().await
}

// logging has to be setup manually before we luanch the containers so that we get log
// output from testcontainers - we can't load Config yet because we don't know the
// database URL until the containers are launched
fn setup_logging() {
    dotenv::dotenv().ok();

    let filter = std::env::var_os("WATERWHEEL_LOG")
        .unwrap_or("info".into());
    let filter = filter
        .to_str()
        .expect("WATERWHEEL_LOG env var is not UTF-8");

    waterwheel::logging::setup_raw(false, filter).expect("failed to setup logging");
}