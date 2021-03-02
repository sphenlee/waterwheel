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

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    logging::setup();

    let app = clap::App::new("waterwheel")
        .author("Steve Lee <sphen.lee@gmail.com>")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(clap::App::new("server"))
        .subcommand(clap::App::new("worker"));

    let args = app.get_matches();

    match args.subcommand() {
        ("server", Some(_args)) => server::run_server().await,
        ("worker", Some(_args)) => worker::run_worker().await,
        _ => unreachable!("clap should have already checked the subcommands"),
    }
}
