#![feature(never_type)]

use anyhow::Result;
use async_std::future::Future;
use async_std::task;
use futures::TryFutureExt;

mod amqp;
mod db;

pub mod messages;
mod server;
mod worker;

pub fn spawn_and_log(name: &str, future: impl Future<Output = Result<!>> + Send + 'static) {
    let _ = async_std::task::Builder::new()
        .name(name.to_owned())
        .spawn(async { future.map_err(|err| panic!("task failed: {}", err)).await })
        .expect("spawn failed");
}

fn main() -> Result<()> {
    dotenv::dotenv().ok();
    //env_logger::builder().format_timestamp_millis().init();
    tide::log::start();

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
