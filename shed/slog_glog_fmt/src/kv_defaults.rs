/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module defining FacebookKV structure that should be used by all Facebook services

use anyhow::Result;
use slog::{Record, Result as SlogResult, Serializer, KV};

/// Structure containing all common KV values for Facebook services
pub struct FacebookKV {
    hostname: String,
}

impl FacebookKV {
    /// Creates a new instance of this structure
    pub fn new() -> Result<Self> {
        Ok(FacebookKV {
            hostname: hostname::get()?.to_string_lossy().into_owned(),
        })
    }
}

impl KV for FacebookKV {
    fn serialize(&self, _record: &Record<'_>, serializer: &mut dyn Serializer) -> SlogResult {
        serializer.emit_str("hostname", &self.hostname)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use slog::{b, record, Level};

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
        assert!(result[0].1 != "");
    }
}
