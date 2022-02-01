use crate::config;
use anyhow::Result;
use chrono::SecondsFormat;
use colored::Colorize;
use std::fmt::{Debug, Result as FmtResult, Write};
use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::field::RecordFields;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::prelude::*;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{fmt, EnvFilter};

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
            "message" => self.message = format!("{:?}\n", value),
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
        writer: &mut dyn Write,
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

        ctx.field_format().format_fields(writer, event)?;

        Ok(())
    }
}

impl<'w> FormatFields<'w> for SemiCompact {
    fn format_fields<R: RecordFields>(&self, writer: &'w mut dyn Write, fields: R) -> FmtResult {
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

pub fn setup() -> Result<()> {
    let use_json = config::get().json_log;

    let filter_layer = EnvFilter::new(&config::get().log);

    if use_json {
        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt::layer().json())
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
