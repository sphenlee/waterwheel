use crate::config;
use cadence::{BufferedUdpMetricSink, NopMetricSink, QueuingMetricSink, StatsdClient};
use once_cell::sync::Lazy;
use std::net::UdpSocket;
use tracing::warn;

const METRIC_PREFIX: &str = "waterwheel"; // TODO - customise this for multiple deployments

static STATSD_CLIENT: Lazy<StatsdClient> =
    Lazy::new(|| match config::get().statsd_server.as_deref() {
        Some(server) => {
            let socket = UdpSocket::bind("0.0.0.0:0").expect("failed to bind to statsd socket");
            let sink = QueuingMetricSink::from(
                BufferedUdpMetricSink::from(server, socket)
                    .expect("failed to create UdpMetricSink"),
            );

            StatsdClient::builder(METRIC_PREFIX, sink).build()
        }
        None => {
            warn!("not sending metrics");
            StatsdClient::builder(METRIC_PREFIX, NopMetricSink).build()
        }
    });

pub fn get_client() -> StatsdClient {
    STATSD_CLIENT.clone()
}
