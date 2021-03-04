use anyhow::{Error, Result};
use std::fmt::Debug;
use std::str::FromStr;

pub fn get<T>(key: &str) -> Result<T>
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    let val =
        std::env::var(key).map_err(|err| Error::msg(format!("error getting {}: {}", key, err)))?;
    let t = val
        .parse()
        .map_err(|err| Error::msg(format!("error parsing {}: {:?}", key, err)))?;
    Ok(t)
}

pub fn get_or<T, D>(key: &str, default: D) -> T
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
    D: Into<T>,
{
    match get(key) {
        Ok(val) => val,
        Err(_) => default.into(),
    }
}
