/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use mysql_async::Value;
use mysql_async::prelude::ToValue;

use crate::mysql::AsSql;

/// A query parameter, formatted as SQL for MySQL or bound as a value for SQLite.
pub trait QueryParameter: AsSql {
    /// Convert this parameter to the value representation used for SQLite binding.
    fn to_sqlite_value(&self) -> Value;
}

impl<T: ToValue> QueryParameter for T {
    fn to_sqlite_value(&self) -> Value {
        ToValue::to_value(self)
    }
}

/// Bytes explicitly treated as a binary string, even if they contain valid UTF-8.
#[derive(Clone, Copy, Debug)]
pub struct Binary<'a>(pub &'a [u8]);

impl AsSql for Binary<'_> {
    fn as_sql(&self, _no_backslash_escape: bool) -> String {
        format!("X'{}'", hex::encode_upper(self.0))
    }
}

impl QueryParameter for Binary<'_> {
    fn to_sqlite_value(&self) -> Value {
        Value::Bytes(self.0.to_vec())
    }
}

impl AsSql for &Binary<'_> {
    fn as_sql(&self, no_backslash_escape: bool) -> String {
        AsSql::as_sql(*self, no_backslash_escape)
    }
}

impl QueryParameter for &Binary<'_> {
    fn to_sqlite_value(&self) -> Value {
        QueryParameter::to_sqlite_value(*self)
    }
}
