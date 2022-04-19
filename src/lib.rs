#![feature(never_type)]
#![feature(assert_matches)]
#![feature(const_option_ext)]

mod amqp;
pub mod circuit_breaker;
pub mod config;
pub mod counter;
mod db;
pub mod logging;
pub mod messages;
mod metrics;
pub mod postoffice;
pub mod server;
pub mod util;
pub mod worker;

pub const GIT_VERSION: &str = git_version::git_version!(fallback = option_env!("GIT_HASH").unwrap_or("NONE"));
