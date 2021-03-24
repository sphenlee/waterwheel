#![feature(never_type)]

use anyhow::Result;

mod amqp;
pub mod circuit_breaker;
pub mod config;
mod db;
mod logging;
pub mod messages;
pub mod postoffice;
mod server;
pub mod util;
mod worker;
mod metrics;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    logging::setup();

    let app = clap::App::new("waterwheel")
        .author("Steve Lee <sphen.lee@gmail.com>")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(clap::App::new("scheduler"))
        .subcommand(clap::App::new("api"))
        .subcommand(clap::App::new("server"))
        .subcommand(clap::App::new("worker"));

    let args = app.get_matches();

    match args.subcommand() {
        ("scheduler", Some(_args)) => {
            db::create_pool().await?;
            amqp::amqp_connect().await?;
            server::run_scheduler().await
        },
        ("api", Some(_args)) => {
            db::create_pool().await?;
            amqp::amqp_connect().await?;
            server::run_api().await
        },
        ("server", Some(_args)) => {
            db::create_pool().await?;
            amqp::amqp_connect().await?;
            tokio::spawn(server::run_scheduler());
            server::run_api().await
        }
        ("worker", Some(_args)) => worker::run_worker().await,
        _ => unreachable!("clap should have already checked the subcommands"),
    }
}
