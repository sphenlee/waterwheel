use super::State;
use sqlx::PgPool;
use highnoon::Request;

// extension methods for State
pub trait RequestExt {
    fn get_pool(&self) -> PgPool;
}

impl RequestExt for Request<State> {
    fn get_pool(&self) -> PgPool {
        self.state().pool.clone()
    }
}
