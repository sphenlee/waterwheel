use anyhow::{Error, Result};
use std::fmt::Debug;
use std::str::FromStr;

pub fn get<T>(key: &str) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    get_opt(key)?.ok_or_else(|| Error::msg(format!("environment variable {} not found", key)))
}

pub fn get_opt<T>(key: &str) -> Result<Option<T>>
    where
        T: FromStr,
        <T as FromStr>::Err: Debug,
{
    match std::env::var(key) {
        Ok(val) => {
            val
                .parse()
                .map_err(|err| Error::msg(format!("error parsing {}: {:?}", key, err)))
                .map(Some)
        },
        Err(std::env::VarError::NotPresent) => {
            Ok(None)
        },
        Err(err) => {
            Err(Error::msg(format!("error getting {}: {}", key, err)))
        }
    }
}

pub fn get_or<T, D>(key: &str, default: D) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
    D: Into<T>,
{
    match get_opt(key)? {
        Some(val) => Ok(val),
        None => Ok(default.into()),
    }
}
