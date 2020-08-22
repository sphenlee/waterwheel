use chrono::SecondsFormat;
use colored::Colorize; // TODO - doesn't support Windows
use env_logger::fmt::Formatter;
use log::Level;
use std::io::Write;

pub fn setup() {
    if std::fs::metadata("log4rs.yaml").is_ok() {
        log4rs::init_file("log4rs.yaml", log4rs::file::Deserializers::default())
            .expect("failed installing log4rs logger");
    } else {
        env_logger::builder().format(env_log_format_with_kv).init();
    }
}

struct Visitor<'a> {
    fmt: &'a mut Formatter,
}

impl<'kvs, 'a> log::kv::Visitor<'kvs> for Visitor<'a> {
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        val: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        writeln!(self.fmt, "    {}: {}", key.to_string().cyan(), val,).unwrap();
        Ok(())
    }
}

fn level_color(level: log::Level, msg: String) -> impl std::fmt::Display {
    match level {
        Level::Error => msg.bright_red(),
        Level::Warn => msg.bright_yellow(),
        Level::Info => msg.bright_white(),
        Level::Debug => msg.bright_green(),
        Level::Trace => msg.bright_purple(),
    }
}

fn env_log_format_with_kv(fmt: &mut Formatter, record: &log::Record) -> std::io::Result<()> {
    let header = format!(
        "[{} {} {}]",
        chrono::Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
        record.level(),
        record.target()
    );

    writeln!(
        fmt,
        "{} {}",
        level_color(record.level(), header),
        record.args()
    )?;

    let mut visitor = Visitor { fmt };
    record.key_values().visit(&mut visitor).unwrap();

    Ok(())
}
