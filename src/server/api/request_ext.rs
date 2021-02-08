use super::State;
use highnoon::Request;
use sqlx::PgPool;

// extension methods for State
pub trait RequestExt {
    fn get_pool(&self) -> PgPool;
}

impl RequestExt for Request<State> {
    fn get_pool(&self) -> PgPool {
        self.state().pool.clone()
    }
}
