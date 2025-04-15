/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::io;
use std::sync;
use std::thread;

use anyhow::Result;
use itertools::Either;
use itertools::Itertools;
use slog::Drain;
use slog::KV;
use slog::Key;
use slog::Level;
use slog::Logger;
use slog::Never;
use slog::OwnedKVList;
use slog::Record;
use slog::o;
use slog_term::Decorator;
use slog_term::PlainSyncDecorator;
use slog_term::RecordDecorator;
use slog_term::TermDecorator;

use crate::collector_serializer::CollectorSerializer;
use crate::kv_categorizer::ErrorCategorizer;
use crate::kv_categorizer::FacebookCategorizer;
use crate::kv_categorizer::KVCategorizer;
use crate::kv_categorizer::KVCategory;
use crate::kv_defaults::FacebookKV;

/// Create a default drain that outputs to stderr and inlines all KV values except for supported
/// error_chain errors.
/// # Example:
/// ```
/// use slog::Logger;
/// use slog::info;
/// use slog::o;
/// use slog_glog_fmt::default_drain;
///
/// fn main() {
///     let logger = Logger::root(default_drain(), o!());
///     info!(logger, "Hello world!");
/// }
/// ```
pub fn default_drain() -> impl Drain<Ok = (), Err = Never> {
    let decorator = TermDecorator::new().build();
    let drain = GlogFormat::new(decorator, ErrorCategorizer).fuse();
    sync::Mutex::new(drain).fuse()
}

/// Create a default root logger for Facebook services
pub fn facebook_logger() -> Result<Logger> {
    let decorator = PlainSyncDecorator::new(io::stderr());
    let drain = GlogFormat::new(decorator, FacebookCategorizer).fuse();
    Ok(Logger::root(drain, o!(FacebookKV::new()?)))
}

/// If you set this as the root logger at the start of your tests
/// you will actually see log lines for failed tests. It looks the same
/// as `facebook_logger`.
/// Note: this may not show logs from threads not under the test runner
/// <https://docs.rs/slog-term/2.8.0/slog_term/struct.TestStdoutWriter.html#note>
pub fn logger_that_can_work_in_tests() -> Result<Logger> {
    let decorator = PlainSyncDecorator::new(slog_term::TestStdoutWriter);
    let drain = GlogFormat::new(decorator, FacebookCategorizer).fuse();
    Ok(Logger::root(drain, o!(FacebookKV::new()?)))
}

/// A slog `Drain` for glog-formatted logs.
pub struct GlogFormat<D: Decorator, C: KVCategorizer> {
    decorator: D,
    categorizer: C,
}

impl<D: Decorator, C: KVCategorizer> GlogFormat<D, C> {
    /// Create a glog-formatted `Drain` using the provided `Decorator`, and `Categorizer`
    pub fn new(decorator: D, categorizer: C) -> GlogFormat<D, C> {
        GlogFormat {
            decorator,
            categorizer,
        }
    }
}

#[cfg(target_os = "linux")]
#[inline(always)]
fn get_tid() -> i32 {
    ::nix::unistd::gettid().as_raw()
}

#[cfg(all(unix, not(target_os = "linux")))]
#[inline(always)]
fn get_tid() -> i32 {
    ::nix::unistd::getpid().as_raw()
}

#[cfg(not(unix))]
#[inline(always)]
fn get_tid() -> i32 {
    0
}

fn write_logline(
    decorator: &mut dyn RecordDecorator,
    level: Level,
    metadata: &OnelineMetadata,
) -> io::Result<()> {
    // Convert log level to a single character representation.
    let level = match level {
        Level::Critical => 'C',
        Level::Error => 'E',
        Level::Warning => 'W',
        Level::Info => 'I',
        Level::Debug => 'V',
        Level::Trace => 'V',
    };

    decorator.start_level()?;
    write!(decorator, "{level}")?;

    decorator.start_timestamp()?;
    write!(decorator, "{}", metadata.now.format("%m%d %H:%M:%S%.6f"))?;

    decorator.start_whitespace()?;
    write!(decorator, " ")?;

    // Write the message.
    decorator.start_msg()?;
    write!(
        decorator,
        "{tid:>5} {tname} {file}:{line}] ",
        tid = metadata.tid,
        tname = metadata.tname,
        file = metadata.file,
        line = metadata.line,
    )
}

fn print_inline_kv<C: KVCategorizer>(
    decorator: &mut dyn RecordDecorator,
    categorizer: &C,
    kv: Vec<(Key, String)>,
) -> io::Result<()> {
    for (k, v) in kv {
        decorator.start_comma()?;
        write!(decorator, ", ")?;
        decorator.start_key()?;
        write!(decorator, "{}", categorizer.name(k))?;
        decorator.start_separator()?;
        write!(decorator, ": ")?;
        decorator.start_value()?;
        write!(decorator, "{v}")?;
    }
    Ok(())
}

fn finish_logline(decorator: &mut dyn RecordDecorator) -> io::Result<()> {
    decorator.start_whitespace()?;
    writeln!(decorator)?;
    decorator.flush()
}

impl<D: Decorator, C: KVCategorizer> Drain for GlogFormat<D, C> {
    type Ok = ();
    type Err = io::Error;

    fn log(&self, record: &Record<'_>, values: &OwnedKVList) -> io::Result<Self::Ok> {
        self.decorator.with_record(record, values, |decorator| {
            let (inline_kv, level_kv): (Vec<_>, Vec<_>) = {
                let mut serializer = CollectorSerializer::new(&self.categorizer);
                values.serialize(record, &mut serializer)?;
                record.kv().serialize(record, &mut serializer)?;

                serializer
                    .into_inner()
                    .into_iter()
                    .filter_map(|(k, v)| match self.categorizer.categorize(k) {
                        KVCategory::Ignore => None,
                        KVCategory::Inline => Some((None, k, v)),
                        KVCategory::LevelLog(level) => Some((Some(level), k, v)),
                    })
                    .partition_map(|(l, k, v)| match l {
                        None => Either::Left((k, v)),
                        Some(level) => Either::Right((level, k, v)),
                    })
            };

            let metadata = OnelineMetadata::new(record);

            write_logline(decorator, record.level(), &metadata)?;
            write!(decorator, "{}", record.msg())?;
            print_inline_kv(decorator, &self.categorizer, inline_kv)?;
            finish_logline(decorator)?;

            for (level, k, v) in level_kv {
                write_logline(decorator, level, &metadata)?;
                write!(decorator, "{}: {}", self.categorizer.name(k), v)?;
                finish_logline(decorator)?;
            }
            Ok(())
        })
    }
}

struct OnelineMetadata {
    now: chrono::DateTime<chrono::Local>,
    tid: i32,
    file: &'static str,
    line: u32,
    tname: String,
}

impl OnelineMetadata {
    fn new(record: &Record<'_>) -> Self {
        OnelineMetadata {
            now: chrono::Local::now(),
            tid: get_tid(),
            file: record.file(),
            line: record.line(),
            tname: thread::current()
                .name()
                .map(|s| format!("[{s}]"))
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::sync::Arc;
    use std::sync::LazyLock;
    use std::sync::Mutex;

    use anyhow::Error;
    use failure_ext::SlogKVError;
    use itertools::assert_equal;
    use regex::Captures;
    use regex::Regex;
    use slog::Drain;
    use slog::Logger;
    use slog::info;
    use slog::o;
    use slog_term::PlainSyncDecorator;
    use thiserror::Error;

    use super::GlogFormat;
    use super::get_tid;
    use crate::kv_categorizer::FacebookCategorizer;
    use crate::kv_categorizer::InlineCategorizer;

    // Create a regex that matches log lines.
    static LOG_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?m)^(.)(\d{4} \d\d:\d\d:\d\d\.\d{6}) +(\d+)(?: \[([\d\S-]+)\] )?([^:]+):(\d+)\] ([^\n]*(?:\n[^IEV][^\n]*)*)$"
        ).unwrap()
    });

    #[derive(Error, Debug)]
    enum TestError {
        #[error("my error #{0} displayed")]
        MyError(usize),
    }

    /// Wrap a buffer so that it can be used by slog as a log output.
    #[derive(Clone)]
    pub struct TestBuffer {
        buffer: Arc<Mutex<Vec<u8>>>,
    }

    impl TestBuffer {
        pub fn new() -> TestBuffer {
            TestBuffer {
                buffer: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn get_string(&self) -> String {
            let buffer = self.buffer.lock().unwrap();
            String::from_utf8(buffer.clone()).unwrap()
        }
    }

    impl io::Write for TestBuffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.buffer.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            self.buffer.lock().unwrap().flush()
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct TestLine {
        level: String,
        tid: String,
        tname: String,
        file: String,
        line: String,
        msg: String,
    }

    impl TestLine {
        fn new(level: &'static str, line: u32, msg: &'static str) -> Self {
            TestLine {
                level: level.to_owned(),
                tid: get_tid().to_string(),
                tname: std::thread::current().name().unwrap().to_string(),
                file: file!().to_owned(),
                line: line.to_string(),
                msg: msg.to_owned(),
            }
        }

        fn with_captures(captures: Captures<'_>) -> Self {
            TestLine {
                level: captures.get(1).unwrap().as_str().to_owned(),
                tid: captures.get(3).unwrap().as_str().to_owned(),
                tname: captures.get(4).unwrap().as_str().to_owned(),
                file: captures.get(5).unwrap().as_str().to_owned(),
                line: captures.get(6).unwrap().as_str().to_owned(),
                msg: captures.get(7).unwrap().as_str().to_owned(),
            }
        }
    }

    #[test]
    fn test_inline() {
        // Create a logger that logs to a buffer instead of stderr.
        let test_buffer = TestBuffer::new();
        let decorator = PlainSyncDecorator::new(test_buffer.clone());
        let drain = GlogFormat::new(decorator, InlineCategorizer).fuse();
        let log = Logger::root(drain, o!("mode" => "test"));

        // Send a log to the buffer. Remember the line the log was on.
        let line = line!() + 1;
        info!(log, "Test log {}", 1; "answer" => 42.42);

        // Get the log string back out of the buffer.
        let log_string = test_buffer.get_string();

        // Check the log line's fields to make sure they match expected values.
        // For the timestamp, it's sufficient to just check it has the right form.
        let captures = LOG_REGEX.captures(log_string.as_str().trim_end()).unwrap();
        assert_eq!(
            TestLine::with_captures(captures),
            TestLine::new("I", line, "Test log 1, mode: test, answer: 42.42",)
        );
    }

    #[test]
    fn test_facebook() {
        // Create a logger that logs to a buffer instead of stderr.
        let test_buffer = TestBuffer::new();
        let decorator = PlainSyncDecorator::new(test_buffer.clone());
        let drain = GlogFormat::new(decorator, FacebookCategorizer).fuse();
        let log = Logger::root(drain, o!("mode" => "test"));

        let err = Error::from(TestError::MyError(0))
            .context(TestError::MyError(1))
            .context(TestError::MyError(2));

        // Send a log to the buffer. Remember the line the log was on.
        let line = line!() + 1;
        info!(log, "Test log {}", 1; "answer" => 42.42, SlogKVError(err));

        // Get the log string back out of the buffer.
        let log_string = test_buffer.get_string();
        let result = LOG_REGEX.find_iter(&log_string).map(|log_line| {
            let log_line = log_line.as_str();
            let captures = LOG_REGEX
                .captures(log_line)
                .unwrap_or_else(|| panic!("failed parsing log line: '{log_line}'"));
            Some(TestLine::with_captures(captures))
        });

        let expected = vec![
            (
                "I",
                "Test log 1, mode: test, Root cause: my error #0 displayed, answer: 42.42",
            ),
            ("E", "Error: my error #2 displayed"),
            ("V",  "Debug context: Error {\n    context: \"my error #2 displayed\",\n    source: Error {\n        context: \"my error #1 displayed\",\n        source: MyError(\n            0,\n        ),\n    },\n}"),
            ("V", "Caused by: my error #1 displayed"),
            ("V", "Caused by: my error #0 displayed"),
        ]
        .into_iter()
        .map(|(level, msg)| TestLine::new(level, line, msg))
        .map(Some);

        assert_equal(result, expected);
    }
}
