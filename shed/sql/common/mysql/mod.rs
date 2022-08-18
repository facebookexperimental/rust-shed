/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
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
pub use facebook::opt_try_from_rowfield;
#[cfg(fbcode_build)]
pub use facebook::Connection;
#[cfg(fbcode_build)]
pub use facebook::ConnectionStats;
#[cfg(fbcode_build)]
pub use facebook::MysqlError;
#[cfg(fbcode_build)]
pub use facebook::OptionalTryFromRowField;
#[cfg(fbcode_build)]
pub use facebook::RowField;
#[cfg(fbcode_build)]
pub use facebook::Transaction;
#[cfg(fbcode_build)]
pub use facebook::TryFromRowField;
#[cfg(fbcode_build)]
pub use facebook::ValueError;
#[cfg(fbcode_build)]
pub use facebook::WriteResult;
pub use mysql_derive::OptTryFromRowField;
pub use mysql_derive::TryFromRowField;
#[cfg(not(fbcode_build))]
pub use mysql_stub::opt_try_from_rowfield;
#[cfg(not(fbcode_build))]
pub use mysql_stub::Connection;
#[cfg(not(fbcode_build))]
pub use mysql_stub::ConnectionStats;
#[cfg(not(fbcode_build))]
pub use mysql_stub::MysqlError;
#[cfg(not(fbcode_build))]
pub use mysql_stub::OptionalTryFromRowField;
#[cfg(not(fbcode_build))]
pub use mysql_stub::RowField;
#[cfg(not(fbcode_build))]
pub use mysql_stub::Transaction;
#[cfg(not(fbcode_build))]
pub use mysql_stub::TryFromRowField;
#[cfg(not(fbcode_build))]
pub use mysql_stub::ValueError;
#[cfg(not(fbcode_build))]
pub use mysql_stub::WriteResult;

use super::WriteResult as SqlWriteResult;

impl From<WriteResult> for SqlWriteResult {
    fn from(result: WriteResult) -> Self {
        Self::new(Some(result.last_insert_id()), result.rows_affected())
    }
}
