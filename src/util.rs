use anyhow::Result;
use sqlx::postgres::PgDatabaseError;
use chrono::Duration;

/// Postgres returns errors in a weird way, sigh
const PG_INTEGRITY_ERROR: &str = "23";

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

/// Check if an error is an integrity error (ie. unique constraint or FK relation failed)
pub fn is_pg_integrity_error(err: &PgDatabaseError) -> bool {
    &err.code()[..2] == PG_INTEGRITY_ERROR
}

/// Format a date in human readable format, but only approx
/// (ie. rounds off everything subsecond)
/// Negative durations will be printed using their absolute length
pub fn format_duration_approx(duration: Duration) -> String {
    let rounded = std::time::Duration::from_secs(duration.num_seconds().unsigned_abs());
    format!("{}", humantime::format_duration(rounded))
}
