use anyhow::Result;
use waterwheel::{config, logging, server::Server, worker::Worker};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();

    let app = clap::Command::new("waterwheel")
        .author("Steve Lee <sphen.lee@gmail.com>")
        .version(waterwheel::GIT_VERSION)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            clap::Arg::new("config_path")
                .long("config")
                .short('c')
                .takes_value(true)
                .help("Provide a specific config file"),
        )
        .subcommand(
            clap::Command::new("scheduler")
                .alias("server")
                .about("launch the scheduler process")
                .after_help("The scheduler has an API server embedded."),
        )
        .subcommand(
            clap::Command::new("api")
                .about("launch the API server process")
                .after_help("The API server may be launched many times for load balancing and HA"),
        )
        .subcommand(clap::Command::new("worker").about("launch the worker process"));

    let args = app.get_matches();

    let config_path = args.value_of("config_path").map(AsRef::as_ref);

    let config = config::load(config_path)?;
    logging::setup(&config)?;

    match args.subcommand().expect("subcommand is required") {
        ("scheduler", _args) => {
            let server = Server::new(config).await?;
            server.run_scheduler().await?;
        }
        ("api", _args) => {
            let server = Server::new(config).await?;
            server.run_api().await?;
        }
        ("worker", _args) => {
            let worker = Worker::new(config).await?;
            worker.run_worker().await?;
        }
        _ => unreachable!("clap should have already checked the subcommands"),
    }
}
