use std::fmt::Formatter;
use crate::worker::engine::TaskEngine;
use anyhow::{Context, Result};
use config::{builder::DefaultState, ConfigBuilder, Environment, File, FileFormat};
use reqwest::Url;
use std::path::Path;
use serde::{Deserialize, Deserializer, de};

struct DurationError(humantime::DurationError);

impl de::Expected for DurationError {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        write!(formatter, "a human duration string ({})", self.0)
    }
}

fn serde_human_time<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<u64, D::Error> {
    let raw: String = Deserialize::deserialize(d)?;
    let secs = humantime::parse_duration(&raw)
        .map_err(|err| {
            de::Error::invalid_value(de::Unexpected::Str(&raw), &DurationError(err))
        })?.as_secs();
    Ok(secs)
}

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

    #[serde(deserialize_with="serde_human_time")]
    pub requeue_interval: u64,

    pub requeue_missed_heartbeats: u32,

    #[serde(deserialize_with="serde_human_time")]
    pub default_task_timeout: u64,

    #[serde(deserialize_with="serde_human_time")]
    pub default_task_retry_delay: u64,

    #[serde(deserialize_with="serde_human_time")]
    pub task_heartbeat: u64,

    #[serde(deserialize_with="serde_human_time")]
    pub log_retention: u64,

    #[serde(deserialize_with="serde_human_time")]
    pub amqp_consumer_timeout: u64,
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
