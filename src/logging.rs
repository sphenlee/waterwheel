use crate::config::Config;
use anyhow::Result;
use chrono::SecondsFormat;
use colored::Colorize;
use std::fmt::{Debug, Result as FmtResult};
use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{
    EnvFilter,
    field::RecordFields,
    fmt::{self, FmtContext, FormatEvent, FormatFields, FormattedFields, format::Writer},
    prelude::*,
    registry::LookupSpan,
};

fn level_color(level: Level, msg: String) -> impl std::fmt::Display {
    match level {
        Level::ERROR => msg.bright_red(),
        Level::WARN => msg.bright_yellow(),
        Level::INFO => msg.bright_green(),
        Level::DEBUG => msg.bright_blue(),
        Level::TRACE => msg.bright_purple(),
    }
}

struct SemiCompactVisitor {
    fields: String,
    message: String,
}

impl Visit for SemiCompactVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        match field.name() {
            "message" => self.message = format!("{value:?}\n"),
            name if name.starts_with("log.") => (),
            name => {
                self.fields
                    .push_str(&format!("    {}: {:?}\n", name.cyan(), value));
            }
        };
    }
}

struct SemiCompact;

impl<C, N> FormatEvent<C, N> for SemiCompact
where
    C: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, C, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> FmtResult {
        let normalized_meta = event.normalized_metadata();
        let meta = normalized_meta.as_ref().unwrap_or_else(|| event.metadata());

        let header = format!(
            "[{} {} {}]",
            chrono::Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            meta.level(),
            meta.target(),
        );

        writeln!(writer, "{}", level_color(*meta.level(), header))?;

        ctx.field_format().format_fields(writer.by_ref(), event)?;

        ctx.visit_spans(|span| {
            //write!(writer, "    -> {}\n", span.name().bold())?;
            let ext = span.extensions();
            let data = ext.get::<FormattedFields<SemiCompact>>().unwrap();
            write!(writer, "{data}")
        })?;

        Ok(())
    }
}

impl<'w> FormatFields<'w> for SemiCompact {
    fn format_fields<R: RecordFields>(&self, mut writer: Writer<'w>, fields: R) -> FmtResult {
        let mut visitor = SemiCompactVisitor {
            fields: String::new(),
            message: String::new(),
        };
        fields.record(&mut visitor);
        write!(writer, "{}", visitor.message.bright_white())?;
        write!(writer, "{}", visitor.fields)?;
        Ok(())
    }
}

pub fn setup(config: &Config) -> Result<()> {
    setup_raw(config.json_log, &config.log)
}

pub fn setup_raw(use_json: bool, filter: &str) -> Result<()> {
    let filter_layer = EnvFilter::new(filter);

    if use_json {
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt::layer().json().with_file(true).with_line_number(true))
            .init();
    } else {
        let fmt_layer = fmt::layer()
            .event_format(SemiCompact)
            .fmt_fields(SemiCompact);

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    }

    Ok(())
}
