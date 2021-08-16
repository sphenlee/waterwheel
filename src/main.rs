#![feature(never_type)]

use anyhow::Result;

mod amqp;
pub mod circuit_breaker;
pub mod config;
mod db;
mod logging;
pub mod messages;
mod metrics;
pub mod postoffice;
mod server;
pub mod util;
mod worker;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    config::load()?;
    logging::setup()?;

    let app = clap::App::new("waterwheel")
        .author("Steve Lee <sphen.lee@gmail.com>")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            clap::App::new("scheduler")
                .alias("server")
                .about("launch the scheduler process")
                .after_help(
                    "There should only be one scheduler active at a time.
                         The scheduler has an API server embedded.",
                ),
        )
        .subcommand(
            clap::App::new("api")
                .about("launch the API server process")
                .after_help("The API server may be launched many times for load balancing and HA"),
        )
        .subcommand(clap::App::new("worker").about("launch the worker process"));

    let args = app.get_matches();

    match args.subcommand() {
        ("scheduler", Some(_args)) => {
            db::create_pool().await?;
            amqp::amqp_connect().await?;

            server::run_scheduler().await?;
            server::run_api().await
        }
        ("api", Some(_args)) => {
            db::create_pool().await?;
            amqp::amqp_connect().await?;

            server::run_api().await
        }
        ("worker", Some(_args)) => worker::run_worker().await,
        _ => unreachable!("clap should have already checked the subcommands"),
    }
}
