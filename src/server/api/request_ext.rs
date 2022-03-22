use super::State;
use cadence::StatsdClient;
use highnoon::Request;
use lapin::Channel;
use sqlx::PgPool;

// extension methods for State
pub trait RequestExt {
    fn get_pool(&self) -> PgPool;
    fn get_channel(&self) -> &Channel;
    fn get_statsd(&self) -> &StatsdClient;
}

impl RequestExt for Request<State> {
    fn get_pool(&self) -> PgPool {
        self.state().server.db_pool.clone()
    }

    fn get_channel(&self) -> &Channel {
        &self.state().amqp_channel
    }

    fn get_statsd(&self) -> &StatsdClient {
        &self.state().server.statsd
    }
}
