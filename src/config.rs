use crate::worker::engine::TaskEngine;
use anyhow::{Context, Result};
use config::{builder::DefaultState, ConfigBuilder, Environment, File, FileFormat};
use reqwest::Url;

/// config for Waterwheel
/// note that the default values are loaded from default_config.toml,
/// mandatory values are not Option *and* not present in that file
#[derive(serde::Deserialize, Clone)]
pub struct Config {
    pub db_url: String, // mandatory
    pub amqp_addr: String,
    pub server_addr: String, // mandatory
    pub server_bind: String,
    pub worker_bind: String,
    pub max_tasks: u32,
    pub task_engine: TaskEngine,
    pub hmac_secret: Option<String>,
    pub public_key: Option<String>,
    pub private_key: Option<String>,
    pub opa_sidecar_addr: Option<Url>,
    #[serde(default)]
    pub no_authz: bool,
    pub statsd_server: Option<String>,
    pub json_log: bool,
    pub log: String,
}

pub fn loader() -> ConfigBuilder<DefaultState> {
    let builder = config::Config::builder();

    builder
        .add_source(File::from_str(
            include_str!("default_config.toml"),
            FileFormat::Toml,
        ))
        .add_source(File::with_name("waterwheel").required(false))
        .add_source(Environment::with_prefix("WATERWHEEL"))
}

pub fn load() -> Result<Config> {
    let config = loader()
        .build()?
        .try_deserialize()
        .context("mandatory configuration value not set")?;

    Ok(config)
}
