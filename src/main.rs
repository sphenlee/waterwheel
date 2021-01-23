#![feature(never_type)]

use anyhow::Result;
use async_std::future::Future;
use async_std::task;
use chrono::Duration;
use circuit_breaker::CircuitBreaker;
use log::error;

mod amqp;
pub mod circuit_breaker;
mod db;
mod logging;
pub mod messages;
pub mod postoffice;
mod server;
pub mod util;
mod worker;

pub fn spawn_retry<F, Fut>(name: impl Into<String>, func: F)
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<!>> + Send + 'static,
{
    let name = name.into();

    let _ = async_std::task::Builder::new()
        .name(name.clone())
        .spawn(async move {
            let mut cb = CircuitBreaker::new(5, Duration::minutes(1));
            while cb.retry() {
                match func().await {
                    Ok(_) => unreachable!("func never returns"),
                    Err(err) => error!("task {} failed: {}", name, err),
                }
            }
            error!("task {} failed too many times, aborting!", name);
            async_std::process::exit(1);
        })
        .expect("spawn failed");
}

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    logging::setup();

    let app = clap::App::new("waterwheel")
        .author("Steve Lee <sphen.lee@gmail.com>")
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        .subcommand(clap::App::new("server"))
        .subcommand(clap::App::new("worker"));

    let args = app.get_matches();

    match args.subcommand() {
        ("server", Some(_args)) => task::block_on(server::run_server()),
        ("worker", Some(_args)) => task::block_on(worker::run_worker()),
        _ => unreachable!("clap should have already checked the subcommands"),
    }
}
