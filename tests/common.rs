use std::future::Future;
use testcontainers::Docker;

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

    f().await
}
