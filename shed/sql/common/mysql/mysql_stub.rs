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

use crate::QueryTelemetry;
use crate::mysql::IsolationLevel;
use crate::mysql::TransactionResult;
use crate::mysql::WriteResult;

/// Error for Mysql client
#[derive(Error, Debug)]
pub struct MysqlError;

impl Display for MysqlError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "MysqlError")
    }
}

/// Connection object.
#[derive(Clone)]
pub struct Connection;

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
    pub async fn read_query<T>(
        &self,
        _query: String,
    ) -> Result<(T, Option<QueryTelemetry>), MysqlError> {
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
