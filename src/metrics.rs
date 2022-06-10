use crate::config::Config;
use anyhow::Result;
use cadence::{BufferedUdpMetricSink, NopMetricSink, QueuingMetricSink, StatsdClient};
use std::net::UdpSocket;
use std::sync::Arc;
use tracing::warn;

const METRIC_PREFIX: &str = "waterwheel"; // TODO - customise this for multiple deployments

pub fn new_client(config: &Config) -> Result<Arc<StatsdClient>> {
    let client = match config.statsd_server.as_deref() {
        Some(server) => {
            let socket = UdpSocket::bind("0.0.0.0:0")?;
            let sink = QueuingMetricSink::from(BufferedUdpMetricSink::from(server, socket)?);

            StatsdClient::builder(METRIC_PREFIX, sink).build()
        }
        None => {
            warn!("not sending metrics");
            StatsdClient::builder(METRIC_PREFIX, NopMetricSink).build()
        }
    };

    Ok(Arc::new(client))
}
