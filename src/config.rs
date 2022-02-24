use crate::worker::engine::TaskEngine;
use anyhow::{Context, Result};
use config as config_loader;
use reqwest::Url;

#[derive(serde::Deserialize, Clone)]
pub struct Config {
    pub db_url: String,
    pub amqp_addr: String,
    pub server_addr: String,
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

pub fn load() -> Result<Config> {
    let mut loader = config_loader::Config::new();

    loader
        .merge(config_loader::File::from_str(
            include_str!("default_config.toml"),
            config_loader::FileFormat::Toml,
        ))?
        .merge(config_loader::File::with_name("waterwheel").required(false))?
        .merge(config_loader::Environment::with_prefix("WATERWHEEL"))?;

    let config = loader
        .try_into()
        .context("mandatory configuration value not set")?;

    Ok(config)
}
