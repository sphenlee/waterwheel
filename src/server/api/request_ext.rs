use super::State;
use highnoon::Request;
use sqlx::PgPool;
use lapin::Channel;

// extension methods for State
pub trait RequestExt {
    fn get_pool(&self) -> PgPool;
    fn get_channel(&self) -> &Channel;
}

impl RequestExt for Request<State> {
    fn get_pool(&self) -> PgPool {
        self.state().pool.clone()
    }

    fn get_channel(&self) -> &Channel {
        &self.state().channel
    }
}
