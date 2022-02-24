use super::State;
use highnoon::Request;
use lapin::Channel;
use sqlx::PgPool;

// extension methods for State
pub trait RequestExt {
    fn get_pool(&self) -> PgPool;
    fn get_channel(&self) -> &Channel;
}

impl RequestExt for Request<State> {
    fn get_pool(&self) -> PgPool {
        self.state().db_pool.clone()
    }

    fn get_channel(&self) -> &Channel {
        &self.state().amqp_channel
    }
}
