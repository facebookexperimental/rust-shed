/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Compatibility shim between `slog` and `tracing`.
//!
//! This crate should be used as a replacement for `slog`.

pub use slog;
pub use slog::BorrowedKV;
pub use slog::Discard;
pub use slog::Drain;
pub use slog::Duplicate;
pub use slog::KV;
pub use slog::Key;
pub use slog::LOG_LEVEL_NAMES;
pub use slog::Level;
pub use slog::Never;
pub use slog::OwnedKV;
pub use slog::OwnedKVList;
pub use slog::Record;
pub use slog::RecordLocation;
pub use slog::RecordStatic;
pub use slog::Result;
pub use slog::SendSyncRefUnwindSafeDrain;
pub use slog::SendSyncRefUnwindSafeKV;
pub use slog::Serializer;
pub use slog::SingleKV;
pub use slog::Value;
pub use slog::b;
pub use slog::o;
pub use tracing;

#[derive(Clone, Debug)]
pub enum Logger {
    Slog(slog::Logger),
    Tracing,
}

impl Logger {
    pub fn root<D, T>(drain: D, values: OwnedKV<T>) -> Logger
    where
        D: SendSyncRefUnwindSafeDrain<Ok = (), Err = Never> + 'static,
        T: SendSyncRefUnwindSafeKV + 'static,
    {
        Logger::Slog(crate::slog::Logger::root_typed(
            std::sync::Arc::new(drain),
            values,
        ))
    }

    pub fn new<T>(&self, values: slog::OwnedKV<T>) -> Logger
    where
        T: slog::SendSyncRefUnwindSafeKV + 'static,
    {
        match self {
            Logger::Slog(logger) => Logger::Slog(logger.new(values)),
            Logger::Tracing => Logger::Tracing,
        }
    }

    pub fn log(&self, record: &Record) {
        match self {
            Logger::Slog(logger) => logger.log(record),
            Logger::Tracing => {}
        }
    }

    pub fn is_enabled(&self, slog_level: Level) -> bool {
        match self {
            Logger::Slog(logger) => logger.is_enabled(slog_level),
            Logger::Tracing => match slog_level {
                Level::Critical => false,
                Level::Error => tracing::enabled!(tracing::Level::ERROR),
                Level::Warning => tracing::enabled!(tracing::Level::WARN),
                Level::Info => tracing::enabled!(tracing::Level::INFO),
                Level::Debug => tracing::enabled!(tracing::Level::DEBUG),
                Level::Trace => tracing::enabled!(tracing::Level::TRACE),
            },
        }
    }
}

pub trait IntoSlogLogger {
    fn into_slog_logger(&self) -> Option<&slog::Logger>;
}

impl IntoSlogLogger for slog::Logger {
    fn into_slog_logger(&self) -> Option<&slog::Logger> {
        Some(self)
    }
}

impl IntoSlogLogger for Logger {
    fn into_slog_logger(&self) -> Option<&slog::Logger> {
        match self {
            Logger::Slog(logger) => Some(logger),
            Logger::Tracing => None,
        }
    }
}
impl IntoSlogLogger for &Logger {
    fn into_slog_logger(&self) -> Option<&slog::Logger> {
        match self {
            Logger::Slog(logger) => Some(logger),
            Logger::Tracing => None,
        }
    }
}
impl IntoSlogLogger for &&Logger {
    fn into_slog_logger(&self) -> Option<&slog::Logger> {
        match self {
            Logger::Slog(logger) => Some(logger),
            Logger::Tracing => None,
        }
    }
}
impl IntoSlogLogger for std::sync::Arc<Logger> {
    fn into_slog_logger(&self) -> Option<&slog::Logger> {
        match self.as_ref() {
            Logger::Slog(logger) => Some(logger),
            Logger::Tracing => None,
        }
    }
}

impl From<slog::Logger> for Logger {
    fn from(logger: slog::Logger) -> Self {
        Logger::Slog(logger)
    }
}

#[macro_export]
macro_rules! compat_tracing_event {
    ( $tracing_level:expr; $( $msg:expr ),*; [ $( $tracing_kv:tt )* ]; [ ] ) => {
        $crate::tracing::event!( $tracing_level, $( $tracing_kv )* $( $msg ),* );
    };
    ( $tracing_level:expr; $( $msg:expr ),*; [ $( $tracing_kv:tt )* ]; [ $key:expr => %$value:expr ] ) => {
        $crate::compat_tracing_event!( $tracing_level; $( $msg ),*; [ $( $tracing_kv )* $key = %$value, ]; [ ] )
    };
    ( $tracing_level:expr; $( $msg:expr ),*; [ $( $tracing_kv:tt )* ]; [ $key:expr => ?$value:expr ] ) => {
        $crate::compat_tracing_event!( $tracing_level; $( $msg ),*; [ $( $tracing_kv )* $key = ?$value, ]; [ ] )
    };
    ( $tracing_level:expr; $( $msg:expr ),*; [ $( $tracing_kv:tt )* ]; [ $key:expr => $value:expr ] ) => {
        $crate::compat_tracing_event!( $tracing_level; $( $msg ),*; [ $( $tracing_kv )* $key = $value, ]; [ ] )
    };
    ( $tracing_level:expr; $( $msg:expr ),*; [ $( $tracing_kv:tt )* ]; [ $key:expr => %$value:expr, $( $kv:tt )* ] ) => {
        $crate::compat_tracing_event!( $tracing_level; $( $msg ),*; [ $( $tracing_kv )* $key = %$value, ]; [ $( $kv )* ] )
    };
    ( $tracing_level:expr; $( $msg:expr ),*; [ $( $tracing_kv:tt )* ]; [ $key:expr => ?$value:expr, $( $kv:tt )* ] ) => {
        $crate::compat_tracing_event!( $tracing_level; $( $msg ),*; [ $( $tracing_kv )* $key = ?$value, ]; [ $( $kv )* ] )
    };
    ( $tracing_level:expr; $( $msg:expr ),*; [ $( $tracing_kv:tt )* ]; [ $key:expr => $value:expr, $( $kv:tt )* ] ) => {
        $crate::compat_tracing_event!( $tracing_level; $( $msg ),*; [ $( $tracing_kv )* $key = $value, ]; [ $( $kv )* ] )
    };
}

#[macro_export]
macro_rules! event {
    ($logger:expr => $slog_level:expr, $tracing_level:expr, $slog_tag:expr; $( $msg:expr ),* $(,)? $( ; $( $kv:tt )* )? ) => {
        if let Some(logger) = $crate::IntoSlogLogger::into_slog_logger(&$logger).as_ref() {
            $crate::slog::log!(logger, $slog_level, $slog_tag, $( $msg ),* $( ; $( $kv )* )? );
        } else {
            $crate::compat_tracing_event!( $tracing_level; $( $msg ),*; [ ]; [ $( $( $kv )* )? ] );
        }
    };
}

#[macro_export]
macro_rules! error {
    ($logger:expr, #$slog_tag:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Error, $crate::tracing::Level::ERROR, $slog_tag; $( $item )* )
    };
    ($logger:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Error, $crate::tracing::Level::ERROR, ""; $( $item )* )
    };
}

#[macro_export]
macro_rules! warn {
    ($logger:expr, #$slog_tag:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Warning, $crate::tracing::Level::WARN, $slog_tag; $( $item )* )
    };
    ($logger:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Warning, $crate::tracing::Level::WARN, ""; $( $item )* )
    };
}

#[macro_export]
macro_rules! info {
    ($logger:expr, #$slog_tag:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Info, $crate::tracing::Level::INFO, $slog_tag; $( $item )* )
    };
    ($logger:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Info, $crate::tracing::Level::INFO, ""; $( $item )* )
    }
}

#[macro_export]
macro_rules! debug {
    ($logger:expr, #$slog_tag:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Debug, $crate::tracing::Level::DEBUG, $slog_tag; $( $item )* )
    };
    ($logger:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Debug, $crate::tracing::Level::DEBUG, ""; $( $item )* )
    };
}

#[macro_export]
macro_rules! trace {
    ($logger:expr, #$slog_tag:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Trace, $crate::tracing::Level::TRACE, $slog_tag; $( $item )* )
    };
    ($logger:expr, $( $item:tt )*) => {
        $crate::event!($logger => $crate::slog::Level::Trace, $crate::tracing::Level::TRACE, ""; $( $item )* )
    };
}
