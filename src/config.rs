use crate::worker::TaskEngine;
use anyhow::{Context, Result};
use config as config_loader;
use once_cell::sync::OnceCell;
use reqwest::Url;

#[derive(serde::Deserialize)]
pub struct Config {
    pub db_url: String,
    pub amqp_addr: String,
    pub redis_url: String,
    pub server_addr: String,
    pub server_bind: String,
    pub worker_bind: String,
    pub max_tasks: u32,
    pub task_engine: TaskEngine,
    pub kube_namespace: String,
    pub hmac_secret: Option<String>,
    pub public_key: Option<String>,
    pub private_key: Option<String>,
    pub opa_sidecar_addr: Option<Url>,
    pub statsd_server: Option<String>,
    pub json_log: bool,
    pub log: String,
}

static CONFIG: OnceCell<Config> = OnceCell::new();

pub fn load() -> Result<()> {
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
    let _ = CONFIG.set(config);
    Ok(())
}

pub fn get() -> &'static Config {
    CONFIG.get().expect("config has not been loaded yet!")
}
