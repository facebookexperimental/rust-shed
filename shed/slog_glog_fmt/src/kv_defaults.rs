/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module defining FacebookKV structure that should be used by all Facebook services

use anyhow::Result;
use slog::KV;
use slog::Record;
use slog::Result as SlogResult;
use slog::Serializer;

/// Structure containing all common KV values for Facebook services
pub struct FacebookKV {
    hostname: String,
    key: &'static str,
}

impl FacebookKV {
    /// Creates a new instance of this structure
    pub fn new() -> Result<Self> {
        Ok(FacebookKV {
            hostname: hostname::get()?.to_string_lossy().into_owned(),
            key: "hostname",
        })
    }

    /// Creates a new instance of this structure, with an overridden key string
    pub fn with_key(key: &'static str) -> Result<Self> {
        Ok(FacebookKV {
            hostname: hostname::get()?.to_string_lossy().into_owned(),
            key,
        })
    }
}

impl KV for FacebookKV {
    fn serialize(&self, _record: &Record<'_>, serializer: &mut dyn Serializer) -> SlogResult {
        serializer.emit_str(self.key, &self.hostname)
    }
}

#[cfg(test)]
mod tests {
    use slog::Level;
    use slog::b;
    use slog::record;

    use super::*;
    use crate::collector_serializer::CollectorSerializer;
    use crate::kv_categorizer::InlineCategorizer;

    #[test]
    fn test() {
        let categorizer = InlineCategorizer;
        let mut serializer = CollectorSerializer::new(&categorizer);
        FacebookKV::new()
            .expect("failed to create kv")
            .serialize(
                &record!(Level::Info, "test", &format_args!(""), b!()),
                &mut serializer,
            )
            .expect("failed to serialize");
        let result = serializer.into_inner();
        assert!(result.len() == 1);
        assert_eq!(result[0].0, "hostname");
        assert!(!result[0].1.is_empty());
    }

    #[test]
    fn test_with_key() {
        let categorizer = InlineCategorizer;
        let mut serializer = CollectorSerializer::new(&categorizer);
        FacebookKV::with_key("funny")
            .expect("failed to create kv")
            .serialize(
                &record!(Level::Info, "test", &format_args!(""), b!()),
                &mut serializer,
            )
            .expect("failed to serialize");
        let result = serializer.into_inner();
        assert!(result.len() == 1);
        assert_eq!(result[0].0, "funny");
        assert!(!result[0].1.is_empty());
    }
}
