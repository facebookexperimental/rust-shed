/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module that defines basic types and traits for implementations of mysql
//! based [crate::Connection].

mod ext;
mod imp;

use std::fmt::Debug;

use anyhow::Error;
use auto_impl::auto_impl;
use futures_ext::BoxFuture;
use mysql_async::{
    Conn, QueryResult as MysqlQueryResult, TextProtocol, Transaction, TransactionOptions,
};

pub use ext::{MysqlConnectionExt, MysqlTransactionExt};

/// Boxed version of [MysqlConnection] that can be used in async code, because
/// it is Send and Sync. It is also the type used in [crate::Connection] enum
pub type BoxMysqlConnection = Box<dyn MysqlConnection + Send + Sync + 'static>;
/// Boxed version of [MysqlTransaction] that can be used in async code, because
/// it is Send and Sync.
pub type BoxMysqlTransaction = Box<dyn MysqlTransaction + Send + Sync + 'static>;

type QueryResult<T> = MysqlQueryResult<T, TextProtocol>;
type QueryProcess<T> =
    Box<dyn FnOnce(QueryResult<T>) -> BoxFuture<QueryResult<T>, Error> + Send + 'static>;

/// Alias for the Boxed function that processes [mysql_async::QueryResult] in
/// [MysqlConnection::query] method.
pub type ConQueryProcess = QueryProcess<Conn>;
/// Alias for the Boxed function that processes [mysql_async::QueryResult] in
/// [MysqlTransaction::query] method.
pub type TraQueryProcess = QueryProcess<Transaction<Conn>>;

/// The main trait of this module, implementations of this trait can be used
/// with the sql's queries macro as part of [crate::Connection].
#[auto_impl(Box)]
pub trait MysqlConnection: Debug {
    /// Perform the string query and process the [mysql_async::QueryResult].
    /// A frequent pattern is to extract some values from the QueryResult and
    /// send it through a [::futures_old::sync::oneshot::channel] to retrieve it.
    fn query(&self, query: String, process: ConQueryProcess) -> BoxFuture<(), Error>;
    /// Return a transaction for this connection with provided options.
    fn transaction_with_options(
        &self,
        options: TransactionOptions,
    ) -> BoxFuture<BoxMysqlTransaction, Error>;
    /// A way for making [BoxMysqlConnection] implement Clone
    fn box_clone(&self) -> BoxMysqlConnection;

    /// Useful shortcut for creating a transaction with default transaction
    /// options.
    fn transaction(&self) -> BoxFuture<BoxMysqlTransaction, Error> {
        self.transaction_with_options(TransactionOptions::new())
    }
}

impl Clone for BoxMysqlConnection {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}

/// A transaction returned from [MysqlConnection::transaction_with_options]
/// have to return a Boxed version of this trait.
pub trait MysqlTransaction {
    /// Perform the string query and process the [mysql_async::QueryResult].
    /// A frequent pattern is to extract some values from the QueryResult and
    /// send it through a [::futures_old::sync::oneshot::channel] to retrieve it.
    fn query(
        self: Box<Self>,
        query: String,
        process: TraQueryProcess,
    ) -> BoxFuture<BoxMysqlTransaction, Error>;
    /// Commit this transaction and drop it.
    fn commit(self: Box<Self>) -> BoxFuture<(), Error>;
    /// Rollback this transaction and drop it.
    fn rollback(self: Box<Self>) -> BoxFuture<(), Error>;
}
