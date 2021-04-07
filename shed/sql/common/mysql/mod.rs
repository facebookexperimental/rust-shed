/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module provides an abstraction layer over Facebook Mysql client.

#[cfg(fbcode_build)]
mod facebook;
#[cfg(not(fbcode_build))]
mod mysql_stub;

#[cfg(fbcode_build)]
pub use facebook::{
    opt_try_from_rowfield, Connection, ConnectionStats, MysqlError, OptionalTryFromRowField,
    RowField, Transaction, WriteResult,
};
pub use mysql_derive::OptTryFromRowField;
#[cfg(not(fbcode_build))]
pub use mysql_stub::{
    opt_try_from_rowfield, Connection, ConnectionStats, MysqlError, OptionalTryFromRowField,
    RowField, Transaction, WriteResult,
};

use super::WriteResult as SqlWriteResult;
use std::convert::Into;

impl Into<SqlWriteResult> for WriteResult {
    fn into(self) -> SqlWriteResult {
        SqlWriteResult::new(Some(self.last_insert_id()), self.rows_affected())
    }
}
