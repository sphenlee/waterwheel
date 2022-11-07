use crate::worker::engine::TaskEngine;
use anyhow::{Context, Result};
use config::{builder::DefaultState, ConfigBuilder, Environment, File, FileFormat};
use reqwest::Url;
use std::path::Path;

/// config for Waterwheel
/// note that the default values are loaded from default_config.toml,
/// mandatory values are not Option *and* not present in that file
#[derive(serde::Deserialize, Clone)]
pub struct Config {
    pub db_url: String, // mandatory
    pub amqp_addr: String,
    pub redis_url: String,
    pub server_addr: String, // mandatory
    pub server_bind: String,
    pub worker_bind: String,
    pub max_tasks: u32,
    pub task_engine: TaskEngine,
    pub hmac_secret: Option<String>,
    pub public_key: Option<String>,
    pub private_key: Option<String>,
    pub opa_sidecar_addr: Option<Url>,
    pub no_authz: bool,
    pub statsd_server: Option<String>,
    pub json_log: bool,
    pub log: String,
    pub cluster_id: Option<String>,
    pub cluster_gossip_bind: String,
    pub cluster_gossip_addr: String,
    pub cluster_seed_nodes: Vec<String>,
    pub requeue_interval_secs: u64, // TODO - deserialise both of these as duration strings
    pub default_task_timeout_secs: u64,
    pub log_retention_secs: usize,
}

pub fn loader(file: Option<&Path>) -> ConfigBuilder<DefaultState> {
    let mut builder = config::Config::builder();

    builder = builder.add_source(File::from_str(
        include_str!("default_config.toml"),
        FileFormat::Toml,
    ));

    if let Some(file) = file {
        builder = builder.add_source(File::from(file));
    }

    builder.add_source(
        Environment::with_prefix("WATERWHEEL")
            .list_separator(",")
            .try_parsing(true)
            .with_list_parse_key("cluster_seed_nodes"),
    )
}

pub fn load(file: Option<&Path>) -> Result<Config> {
    let config = loader(file)
        .build()?
        .try_deserialize()
        .context("mandatory configuration value not set")?;

    Ok(config)
}
