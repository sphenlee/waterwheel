use crate::config;
use anyhow::Result;
use cadence::{BufferedUdpMetricSink, NopMetricSink, QueuingMetricSink, StatsdClient};
use once_cell::sync::Lazy;
use std::net::UdpSocket;
use tracing::warn;
use crate::config::Config;

const METRIC_PREFIX: &str = "waterwheel"; // TODO - customise this for multiple deployments

pub fn new_client(config: &Config) -> Result<StatsdClient> {
    match config.statsd_server.as_deref() {
        Some(server) => {
            let socket = UdpSocket::bind("0.0.0.0:0")?;
            let sink = QueuingMetricSink::from(
                BufferedUdpMetricSink::from(server, socket)?,
            );

            Ok(StatsdClient::builder(METRIC_PREFIX, sink).build())
        }
        None => {
            warn!("not sending metrics");
            Ok(StatsdClient::builder(METRIC_PREFIX, NopMetricSink).build())
        }
    }
}
