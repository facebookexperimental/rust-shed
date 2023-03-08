/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Facebook Mysql client stub.

use std::fmt;
use std::fmt::Display;

use thiserror::Error;

use crate::mysql::IsolationLevel;

/// Error for Mysql client
#[derive(Error, Debug)]
pub struct MysqlError;

impl Display for MysqlError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "MysqlError")
    }
}

/// Value conversion error for Mysql client
#[derive(Error, Debug)]
pub struct ValueError;

impl Display for ValueError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "ValueError")
    }
}

impl From<ValueError> for MysqlError {
    fn from(_: ValueError) -> Self {
        Self
    }
}

/// Result returned by a write query
pub struct WriteResult;

impl WriteResult {
    /// Get last inserted id
    pub fn last_insert_id(&self) -> u64 {
        unimplemented!("This is a stub");
    }
    /// Get number of affected rows
    pub fn rows_affected(&self) -> u64 {
        unimplemented!("This is a stub");
    }
}

/// ODS counters
pub struct ConnectionStats;

/// Connection object.
#[derive(Clone)]
pub struct Connection;

/// Transaction result object.
#[allow(dead_code)]
pub struct TransactionResult<T> {
    results: T,
}

impl<T> TransactionResult<T> {
    /// Get last inserted ids
    pub fn last_insert_id(&self) -> &[u64] {
        unimplemented!("This is a stub");
    }

    /// Get number of affected rows
    pub fn rows_affected(&self) -> u64 {
        unimplemented!("This is a stub");
    }

    /// Get query results.
    pub fn results(&self) -> &T {
        unimplemented!("This is a stub");
    }
}

unsafe impl Send for Connection {}

impl Connection {
    /// Set isolation level for connection
    pub fn set_isolation_level(&mut self, _isolation_level: Option<IsolationLevel>) {
        unimplemented!("This is a stub")
    }

    /// Performs multiple queries in a single transaction.
    #[allow(unused_variables)]
    pub async fn execute_transaction<R, Q>(
        &self,
        queries: impl IntoIterator<Item = Q>,
    ) -> Result<TransactionResult<R>, MysqlError> {
        unimplemented!("This is a stub");
    }

    /// Performs a given query and returns the result as a vector of rows.
    pub async fn read_query<T>(&self, _query: String) -> Result<T, MysqlError> {
        unimplemented!("This is a stub");
    }

    /// Performs a given query and returns the write result.
    pub async fn write_query(&self, _query: String) -> Result<WriteResult, MysqlError> {
        unimplemented!("This is a stub");
    }

    /// Begins trasaction and returns Transaction object.
    pub async fn begin_transaction(&self) -> Result<Transaction, MysqlError> {
        unimplemented!("This is a stub");
    }

    /// Returns the replication lag for a connection.
    pub async fn get_replica_lag_secs(&self) -> Result<Option<u64>, MysqlError> {
        unimplemented!("This is a stub");
    }
}

/// Transaction object.
pub struct Transaction;

impl Transaction {
    /// Performs a given query and returns the result as a vector of rows.
    pub async fn read_query<T>(&mut self, _query: String) -> Result<T, MysqlError> {
        unimplemented!("This is a stub");
    }

    /// Performs a given query and returns the write result.
    pub async fn write_query(&mut self, _query: String) -> Result<WriteResult, MysqlError> {
        unimplemented!("This is a stub");
    }

    /// Commit transaction.
    pub async fn commit(self) -> Result<(), MysqlError> {
        unimplemented!("This is a stub");
    }

    /// Rollback transaction.
    pub async fn rollback(self) -> Result<(), MysqlError> {
        unimplemented!("This is a stub");
    }
}

/// Row field object.
pub struct RowField;

/// The trait you need to implement to be able to read a query result into the custom type.
pub trait OptionalTryFromRowField: Sized {
    /// Try to convert from row field.
    fn try_from_opt(field: RowField) -> Result<Option<Self>, ValueError>;
}

/// The trait you need to implement to be able to read a query result into the custom type where NULL maps to Some
pub trait TryFromRowField: Sized {
    /// Try to convert from row field.
    fn try_from(field: RowField) -> Result<Self, ValueError>;
}

/// The function converts RowField object into Rust type.
pub fn opt_try_from_rowfield<T>(_field: RowField) -> Result<T, ValueError> {
    unimplemented!("This is a stub");
}
