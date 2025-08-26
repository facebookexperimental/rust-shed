/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Module provides an abstraction layer over Facebook Mysql client.

#[cfg(fbcode_build)]
mod facebook;
#[cfg(not(fbcode_build))]
mod mysql_stub;
mod ossmysql_wrapper;
#[cfg(fbcode_build)]
pub use facebook::Connection;
#[cfg(fbcode_build)]
pub use facebook::MysqlError;
#[cfg(fbcode_build)]
pub use facebook::MysqlQueryTelemetry;
#[cfg(fbcode_build)]
pub use facebook::Transaction;
pub use mysql_client_traits::OptionalTryFromRowField;
pub use mysql_client_traits::RowField;
pub use mysql_client_traits::TryFromRowField;
pub use mysql_client_traits::ValueError;
pub use mysql_client_traits::opt_try_from_rowfield;
pub use mysql_derive::OptTryFromRowField;
pub use mysql_derive::TryFromRowField;
#[cfg(not(fbcode_build))]
pub use mysql_stub::Connection;
#[cfg(not(fbcode_build))]
pub use mysql_stub::MysqlError;
#[cfg(not(fbcode_build))]
pub use mysql_stub::Transaction;
pub use ossmysql_wrapper::OssConnection;
pub use ossmysql_wrapper::OssQueryTelemetry;
use stats::prelude::*;

use super::WriteResult as SqlWriteResult;
use crate::QueryTelemetry;

define_stats_struct! {
    ConnectionStats("sql.mysql_ffi.{}", label: String),
    get_connection_ms: histogram(100, 0, 5_000, Average, Count; P 50; P 95; P 99),
    raw_query_ms: histogram(100, 0, 5_000, Average, Count; P 50; P 95; P 99),
}

/// A simple wrapper struct around a SQL string, just to add some type
/// safety.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MySqlQuery {
    query: String,
}

impl MySqlQuery {
    /// Create a new MySqlQuery
    pub fn new(query: impl Into<String>) -> MySqlQuery {
        MySqlQuery {
            query: query.into(),
        }
    }
}

impl std::fmt::Display for MySqlQuery {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.query)
    }
}

/// A trait representing types that can be formatted as SQL.
pub trait AsSql {
    /// Format the given value as a SQL string.
    fn as_sql(&self, no_backslash_escape: bool) -> String;
}

impl AsSql for MySqlQuery {
    fn as_sql(&self, _no_backslash_escape: bool) -> String {
        self.query.clone()
    }
}

impl<T: mysql_async::prelude::ToValue> AsSql for T {
    fn as_sql(&self, no_backslash_escape: bool) -> String {
        mysql_async::prelude::ToValue::to_value(self).as_sql(no_backslash_escape)
    }
}

/// A wrapper around a slice that implements AsSql. Useful for
/// creating IN clauses in `mysql_query!`.
pub struct SqlList<'a, T: AsSql>(&'a [T]);

impl<'a, T: AsSql> SqlList<'a, T> {
    /// Create a new instance of SqlList
    pub fn new(values: &'a [T]) -> SqlList<'a, T> {
        SqlList(values)
    }
}

impl<T: AsSql> AsSql for SqlList<'_, T> {
    fn as_sql(&self, no_backslash_escape: bool) -> String {
        let mut result = String::new();
        result.push('(');
        let mut first = true;
        for value in self.0 {
            if first {
                first = false;
            } else {
                result.push_str(", ");
            }
            result.push_str(&AsSql::as_sql(value, no_backslash_escape));
        }
        result.push(')');
        result
    }
}

/// mysql_query!("SELECT foo FROM table WHERE col = {id}", id = "foo");
/// mysql_query!("SELECT foo FROM table WHERE col = {}", "foo");
/// mysql_query!("SELECT foo FROM table WHERE col1 = {my_var} AND col = {id}", id = "foo"; my_var = "@bar");
#[macro_export]
macro_rules! mysql_query {
    ($query:expr) => {
        ::sql::mysql::MySqlQuery::new(format!($query))
    };
    ($query:expr, $($key:tt = $value:expr),*) => {
        ::sql::mysql::MySqlQuery::new(format!(
                $query,
                $( $key = &::sql::mysql::AsSql::as_sql(&$value, false) ),*
        ))
    };
    ($query:expr, $($arg:expr),*) => {
        ::sql::mysql::MySqlQuery::new(format!(
                $query, $( &::sql::mysql::AsSql::as_sql(&$arg, false) ),*
        ))
    };
    ($query:expr, $($key:ident = $value:expr),* ; $($key2:ident = $variable:expr),*) => {
        ::sql::mysql::MySqlQuery::new(format!(
                $query,
                $( $key = &::sql::mysql::AsSql::as_sql(&$value, false), )*
                $( $key2 = &$variable.as_sql(false), )*
        ))
    };
}
pub use mysql_query;

/// Changes which locks are used. See <https://dev.mysql.com/doc/refman/8.0/en/innodb-transaction-isolation-levels.html>
#[derive(Debug, Clone, Copy)]
pub enum IsolationLevel {
    /// Each consistent read, even within the same transaction, sets and reads its own fresh snapshot.
    ReadCommitted,
}

/// Result returned by a write query
#[cfg(fbcode_build)]
pub struct WriteResult(u64, u64, Option<MysqlQueryTelemetry>);

/// Result returned by a write query
#[cfg(not(fbcode_build))]
pub struct WriteResult(u64, u64, Option<OssQueryTelemetry>);

impl WriteResult {
    /// Create result
    pub fn new(
        last_insert_id: u64,
        rows_affected: u64,
        #[cfg(fbcode_build)] query_telemetry: Option<MysqlQueryTelemetry>,
        #[cfg(not(fbcode_build))] query_telemetry: Option<OssQueryTelemetry>,
    ) -> Self {
        WriteResult(last_insert_id, rows_affected, query_telemetry)
    }

    /// Get last inserted id
    pub fn last_insert_id(&self) -> u64 {
        self.0
    }

    /// Get number of affected rows
    pub fn rows_affected(&self) -> u64 {
        self.1
    }
}

impl From<WriteResult> for SqlWriteResult {
    fn from(result: WriteResult) -> Self {
        Self::new(
            Some(result.last_insert_id()),
            result.rows_affected(),
            result.2.map(QueryTelemetry::from),
        )
    }
}

/// Result returned from executing a transaction.
pub struct TransactionResult<T> {
    last_insert_ids: Vec<u64>,
    rows_affected: u64,
    results: T,
}

impl<T> TransactionResult<T> {
    /// Get last inserted ids
    pub fn last_insert_id(&self) -> &[u64] {
        &self.last_insert_ids
    }

    /// Get number of affected rows
    pub fn rows_affected(&self) -> u64 {
        self.rows_affected
    }

    /// Get query results.
    pub fn results(&self) -> &T {
        &self.results
    }
}
