use once_cell::sync::Lazy;
use log::warn;
use crate::config;
use cadence::{QueuingMetricSink, BufferedUdpMetricSink, StatsdClient, NopMetricSink};
use std::net::UdpSocket;

const METRIC_PREFIX: &str = "waterwheel"; // TODO - customise this for multiple deployments

static STATSD_CLIENT: Lazy<StatsdClient> = Lazy::new(|| {
    match config::get::<String>("WATERWHEEL_STATSD_SERVER") {
        Ok(server) => {
            let socket = UdpSocket::bind("0.0.0.0:0").expect("failed to bind to statsd socket");
            let sink = QueuingMetricSink::from(
                BufferedUdpMetricSink::from(
                server, socket
                ).expect("failed to create UdpMetricSink")
            );

            StatsdClient::builder(METRIC_PREFIX, sink).build()
        }
        Err(err) => {
            warn!("not sending metrics: {}", err);
            StatsdClient::builder(METRIC_PREFIX, NopMetricSink).build()
        }
    }
});

pub fn get_client() -> StatsdClient {
    STATSD_CLIENT.clone()
}
