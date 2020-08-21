#![feature(never_type)]

use anyhow::Result;
use async_std::future::Future;
use async_std::task;
use chrono::SecondsFormat;
use futures::TryFutureExt;
use std::io::Write;

mod amqp;
mod db;
pub mod messages;
pub mod postoffice;
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
    env_logger::builder()
        .format(|fmt, record| {
            writeln!(
                fmt,
                "[{} {} {}] {}",
                chrono::Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
                record.level(),
                record.target(),
                record.args(),
            )?;

            struct Visitor<'a> {
                fmt: &'a mut env_logger::fmt::Formatter,
            }

            impl<'kvs, 'a> log::kv::Visitor<'kvs> for Visitor<'a> {
                fn visit_pair(
                    &mut self,
                    key: log::kv::Key<'kvs>,
                    val: log::kv::Value<'kvs>,
                ) -> Result<(), log::kv::Error> {
                    writeln!(self.fmt, "    {}: {}", key, val).unwrap();
                    Ok(())
                }
            }

            let mut visitor = Visitor { fmt };
            record.key_values().visit(&mut visitor).unwrap();

            Ok(())
        })
        .init();
    //tide::log::with_level(log::LevelFilter::Info);

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
