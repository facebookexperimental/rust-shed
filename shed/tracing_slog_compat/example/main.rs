/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Result;
use tracing::Level;
use tracing_glog::Glog;
use tracing_glog::GlogFields;
use tracing_subscriber::layer::SubscriberExt;

fn setup_tracing() -> Result<()> {
    let event_format = Glog::default()
        .with_timer(tracing_glog::LocalTime::default())
        .with_target(true);

    // Create and register Glog (stderr) logging layer
    let log_layer = tracing_subscriber::fmt::layer()
        .event_format(event_format)
        .fmt_fields(GlogFields::default())
        .with_writer(std::io::stderr)
        .with_ansi(false);
    let subscriber = tracing_subscriber::registry().with(log_layer);

    // Set the subscriber as the global default.
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    setup_tracing()?;

    let logger = match args.get(1).map(String::as_str) {
        None | Some("tracing") => slog::Logger::Tracing,
        Some("slog") => slog::Logger::Slog(slog_glog_fmt::facebook_logger()?),
        Some(other) => {
            anyhow::bail!("Invalid logging type: {other}");
        }
    };

    // Example tracing span:
    let span = tracing::span!(Level::DEBUG, "tracing-span", bin = %"example");
    let _guard = span.enter();

    let var1 = 100;
    let var2 = 200;
    let var3 = 300;
    let dur = std::time::Duration::from_millis(20);
    let string = "example";

    tracing::trace!("Example tracing::trace");
    tracing::debug!(number = 1, word = %"bird");
    tracing::info!(duration = ?dur, "Example tracing::info");
    tracing::warn!("Example tracing::warn");
    tracing::error!(var1, word = %"cat", "Example tracing::error ({} {var3})", var2);

    slog::error!(logger, "Example slog::error");
    slog::warn!(logger, "Example slog::warn"; "key" => "value", "debug" => ?"value", "display" => %"value");
    slog::info!(logger, "Example slog::info {} {var3}", var2; "var1" => var1);
    slog::debug!(logger, "Example slog::debug"; "duration" => ?dur);
    slog::trace!(logger, "Example slog::trace"; "string" => %string);

    Ok(())
}
