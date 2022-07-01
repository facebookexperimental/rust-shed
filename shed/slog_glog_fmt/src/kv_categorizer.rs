/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Provides ways to control how the KV values passed to slog macros are printed

use std::str::FromStr;

use failure_ext::SlogKVErrorKey;
use slog::Key;
use slog::Level;

/// The KV value is being processed based on the category it is bucketed in
#[derive(Debug, PartialEq, Eq)]
pub enum KVCategory {
    /// KV value is not printed at all
    Ignore,
    /// KV value is inlined with the main message passed to slog macro
    Inline,
    /// KV value is printed as a separate line with the provided log level
    LevelLog(Level),
}

/// Structures implementing this trait are being used to categorize the KV values into one of the
/// `KVCategory`.
pub trait KVCategorizer {
    /// For a given key from KV decide which category it belongs to
    fn categorize(&self, key: Key) -> KVCategory;
    /// For a given key from KV return a name that should be printed for it
    fn name(&self, key: Key) -> &'static str;
    /// True if category of a given key is KVCategory::Ignore
    fn ignore(&self, key: Key) -> bool {
        self.categorize(key) == KVCategory::Ignore
    }
}

/// Dummy categorizer that inlines all KV values with names equal to key
pub struct InlineCategorizer;
impl KVCategorizer for InlineCategorizer {
    fn categorize(&self, _key: Key) -> KVCategory {
        KVCategory::Inline
    }

    fn name(&self, key: Key) -> &'static str {
        key
    }
}

/// Used to properly print `error_chain` `Error`s. It displays the error and it's causes in
/// separate log lines as well as backtrace if provided.
/// The `error_chain` `Error` must implement `KV` trait. It is recommended to use `impl_kv_error`
/// macro to generate the implementation.
pub struct ErrorCategorizer;
impl KVCategorizer for ErrorCategorizer {
    fn categorize(&self, key: Key) -> KVCategory {
        match SlogKVErrorKey::from_str(key) {
            Ok(SlogKVErrorKey::Error) => KVCategory::LevelLog(Level::Error),
            Ok(SlogKVErrorKey::Cause) => KVCategory::LevelLog(Level::Debug),
            Ok(SlogKVErrorKey::Backtrace) => KVCategory::LevelLog(Level::Trace),
            Ok(SlogKVErrorKey::RootCause) | Err(()) => InlineCategorizer.categorize(key),
            Ok(SlogKVErrorKey::ErrorDebug) => KVCategory::LevelLog(Level::Debug),
        }
    }

    fn name(&self, key: Key) -> &'static str {
        match SlogKVErrorKey::from_str(key) {
            Ok(SlogKVErrorKey::Error) => "Error",
            Ok(SlogKVErrorKey::Cause) => "Caused by",
            Ok(SlogKVErrorKey::Backtrace) => "Originated in",
            Ok(SlogKVErrorKey::RootCause) => "Root cause",
            Ok(SlogKVErrorKey::ErrorDebug) => "Debug context",
            Err(()) => InlineCategorizer.name(key),
        }
    }
}

/// Categorizer to be used by all Facebook services
pub struct FacebookCategorizer;
impl KVCategorizer for FacebookCategorizer {
    fn categorize(&self, key: Key) -> KVCategory {
        match key {
            "hostname" => KVCategory::Ignore,
            _ => ErrorCategorizer.categorize(key),
        }
    }

    fn name(&self, key: Key) -> &'static str {
        ErrorCategorizer.name(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use anyhow::Error;
    use failure_ext::SlogKVError;
    use itertools::assert_equal;
    use slog::b;
    use slog::record;
    use slog::KV;
    use thiserror::Error;

    use crate::collector_serializer::CollectorSerializer;

    #[derive(Error, Debug)]
    enum TestError {
        #[error("my error #{0} displayed")]
        MyError(usize),
    }

    #[test]
    fn test_inline() {
        let categorizer = InlineCategorizer;
        let values = vec!["test", "test2"];
        for v in values {
            assert_eq!(categorizer.categorize(v), KVCategory::Inline);
            assert_eq!(categorizer.name(v), v);
        }
    }

    #[test]
    fn test_error() {
        let err = Error::from(TestError::MyError(0))
            .context(TestError::MyError(1))
            .context(TestError::MyError(2));
        let debug = format!("{:#?}", err);

        let categorizer = ErrorCategorizer;
        let mut serializer = CollectorSerializer::new(&categorizer);
        SlogKVError(err)
            .serialize(
                &record!(Level::Info, "test", &format_args!(""), b!()),
                &mut serializer,
            )
            .expect("failed to serialize");
        assert_equal(
            serializer.into_inner(),
            vec![
                ("error", "my error #2 displayed".to_owned()),
                ("error_debug", debug),
                ("cause", "my error #1 displayed".to_owned()),
                ("cause", "my error #0 displayed".to_owned()),
                ("root_cause", "my error #0 displayed".to_owned()),
            ],
        );
    }
}
