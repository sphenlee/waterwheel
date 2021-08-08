use anyhow::Result;
use crate::config;
use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::prelude::*;
use tracing_subscriber::fmt::{FormatFields, FormatEvent, FmtContext};
use tracing_subscriber::field::RecordFields;
use chrono::SecondsFormat;
use tracing_subscriber::registry::LookupSpan;
use tracing::{Event, Subscriber, Level};
use tracing::field::{Visit, Field};
use colored::Colorize;
use std::fmt::{Write, Debug, Result as FmtResult};

fn level_color(level: Level, msg: String) -> impl std::fmt::Display {
    match level {
        Level::ERROR => msg.bright_red(),
        Level::WARN => msg.bright_yellow(),
        Level::INFO => msg.bright_green(),
        Level::DEBUG => msg.bright_blue(),
        Level::TRACE => msg.bright_purple(),
    }
}

struct SemiCompactVisitor<'w> {
    writer: &'w mut dyn Write,
    result: FmtResult,
}

impl<'w> Visit for SemiCompactVisitor<'w> {
    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        if self.result.is_err() {
            return;
        }

        self.result = match field.name() {
            "message" => writeln!(self.writer, "{:?}", value),
            name if name.starts_with("log.") => Ok(()),
            name => writeln!(self.writer, "    {}: {:?}", name.cyan(), value)
        };
    }
}

struct SemiCompact;

impl<C, N> FormatEvent<C, N> for SemiCompact
where
    C: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(&self, ctx: &FmtContext<'_, C, N>, writer: &mut dyn Write, event: &Event<'_>) -> FmtResult {
        let header = format!(
            "[{} {} {}]",
            chrono::Local::now().to_rfc3339_opts(SecondsFormat::Millis, false),
            event.metadata().level(),
            event.metadata().target(),
        );

        write!(
            writer,
            "{} ",
            level_color(*event.metadata().level(), header)
        )?;
        /*write!(writer,
            "  (at {}@{})",
            event.metadata().file().unwrap_or("<unknown>"),
            event.metadata().line().unwrap_or(0)
        )?;*/

        ctx.field_format().format_fields(writer, event)?;

        Ok(())
    }
}

impl<'w> FormatFields<'w> for SemiCompact {
    fn format_fields<R: RecordFields>(&self, writer: &'w mut dyn Write, fields: R) -> FmtResult {
        let mut visitor = SemiCompactVisitor {
            writer,
            result: Ok(())
        };
        fields.record(&mut visitor);
        //writeln!(writer)?;
        visitor.result
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
