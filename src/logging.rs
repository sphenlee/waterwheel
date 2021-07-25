use anyhow::Result;
use crate::config;

const DEFAULT_LOG_FILTER: &str = "warn,waterwheel=info";

pub fn setup() -> Result<()> {
    let use_json: bool = config::get_or("WATERWHEEL_JSON_LOG", false)?;

    let mut builder = env_logger::builder();
    builder.format(if use_json {
        json_format::format
    } else {
        env_log_format::format
    });

    builder.parse_env(env_logger::Env::new().filter_or("WATERWHEEL_LOG", DEFAULT_LOG_FILTER));
    builder.init();

    Ok(())
}

mod env_log_format {
    use chrono::SecondsFormat;
    use colored::Colorize; // TODO - doesn't support Windows
    use env_logger::fmt::Formatter;
    use log::Level;
    use std::io::Write;

    struct Visitor<'a> {
        fmt: &'a mut Formatter,
    }

    impl<'kvs, 'a> log::kv::Visitor<'kvs> for Visitor<'a> {
        fn visit_pair(
            &mut self,
            key: log::kv::Key<'kvs>,
            val: log::kv::Value<'kvs>,
        ) -> Result<(), log::kv::Error> {
            writeln!(self.fmt, "    {}: {}", key.to_string().cyan(), val).unwrap();
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

    pub(crate) fn format(fmt: &mut Formatter, record: &log::Record) -> std::io::Result<()> {
        let header = format!(
            "[{} {} {}]",
            chrono::Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            record.level(),
            record.target(),
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
}

mod json_format {
    use env_logger::fmt::Formatter;
    use std::collections::HashMap;
    use std::io::Write;

    #[derive(serde::Serialize)]
    struct JsonRecord<'a> {
        ts: String,
        level: &'static str,
        target: &'a str,
        msg: String,
        extra: HashMap<String, String>,
    }

    struct Visitor<'a> {
        extra: &'a mut HashMap<String, String>,
    }

    impl<'kvs, 'a> log::kv::Visitor<'kvs> for Visitor<'a> {
        fn visit_pair(
            &mut self,
            key: log::kv::Key<'kvs>,
            val: log::kv::Value<'kvs>,
        ) -> Result<(), log::kv::Error> {
            self.extra.insert(key.to_string(), val.to_string());
            Ok(())
        }
    }

    pub(crate) fn format(fmt: &mut Formatter, record: &log::Record) -> std::io::Result<()> {
        let mut json = JsonRecord {
            ts: chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false),
            level: record.level().as_str(),
            target: record.target(),
            msg: record.args().to_string(),
            extra: HashMap::new(),
        };

        let mut visitor = Visitor {
            extra: &mut json.extra,
        };
        record.key_values().visit(&mut visitor).unwrap();

        serde_json::to_writer(&mut *fmt, &json)?;
        fmt.write_all(b"\n")?;

        Ok(())
    }
}
