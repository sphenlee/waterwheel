use anyhow::Result;
use sqlx::postgres::PgDatabaseError;

/// Postgres returns errors in a weird way, sigh
pub const PG_INTEGRITY_ERROR: &str = "23";

/// Converts a SQLx error into a nested error - the outer layer is for all unexpected
/// errors, the inner layer is for Database errors from Postgres. The inner error is
/// downcast into the correct PgDatabaseError type so that it can be checked.
pub fn pg_error<T>(res: sqlx::Result<T>) -> Result<std::result::Result<T, Box<PgDatabaseError>>> {
    match res {
        Ok(t) => Ok(Ok(t)),
        Err(err) => match err {
            sqlx::Error::Database(db_err) => Ok(Err(db_err.downcast::<PgDatabaseError>())),
            err => Err(err.into()),
        },
    }
}
