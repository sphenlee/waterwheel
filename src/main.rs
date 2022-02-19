#![feature(never_type)]
#![feature(assert_matches)]

use anyhow::Result;
use crate::server::Server;
use crate::worker::Worker;

mod amqp;
pub mod circuit_breaker;
pub mod config;
pub mod counter;
mod db;
mod logging;
pub mod messages;
mod metrics;
pub mod postoffice;
mod server;
pub mod util;
mod worker;

pub const GIT_VERSION: &str = git_version::git_version!();

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let app = clap::App::new("waterwheel")
        .author("Steve Lee <sphen.lee@gmail.com>")
        .version(GIT_VERSION)
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
            let server = Server::new().await?;
            server.run_scheduler().await?;
        }
        ("api", Some(_args)) => {
            let server = Server::new().await?;
            server.run_api().await?;
        }
        ("worker", Some(_args)) => {
            let worker = Worker::new().await?;
            worker.run_worker().await?;
        },
        _ => unreachable!("clap should have already checked the subcommands"),
    }
}
