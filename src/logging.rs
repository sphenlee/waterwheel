pub fn setup() {
    if std::env::var("WATERWHEEL_JSON_LOG").is_ok() {
        env_logger::builder().format(json_format::format).init();
    } else {
        env_logger::builder().format(env_log_format::format).init();
    }
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
