/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module that provides support for SQL transactions to this library.

use anyhow::Error;
use cloned::cloned;
use futures::future::TryFutureExt;
use futures_ext::{BoxFuture, FutureExt};
use futures_old::{future::IntoFuture, Future};
use futures_util::FutureExt as NewFutureExt;
use mysql_async::TransactionOptions;

use crate::deprecated_mysql::BoxMysqlTransaction;
use crate::mysql;
use crate::sqlite::SqliteConnectionGuard;

impl crate::Connection {
    /// Start an SQL transaction for this connection. Refer to `transaction::Transaction` docs for
    /// more info
    pub fn start_transaction(&self) -> BoxFuture<Transaction, Error> {
        Transaction::new(self)
    }

    /// Start an SQL transaction for this connection. Refer to `transaction::Transaction` docs for
    /// more info
    pub fn start_transaction_with_options(
        &self,
        options: TransactionOptions,
    ) -> BoxFuture<Transaction, Error> {
        Transaction::new_with_options(self, options)
    }
}

/// Enum for generalizing transactions over Sqlite and MyRouter.
///
/// # Example
/// ```
/// use anyhow::Error;
/// use futures_old::Future;
///
/// use sql::{queries, Connection};
/// use sql_tests_lib::{A, B};
///
/// queries! {
///     read MySelect(param_a: A, param_uint: u64) -> (u64, B, B, i64) {
///         "SELECT 44, NULL, {param_a}, {param_uint}"
///     }
///     write MyInsert(values: (x: i64)) {
///         none,
///         "INSERT INTO foo (x) VALUES ({values})"
///     }
/// }
///
/// fn foo(conn: Connection) -> impl Future<Item=(), Error=Error> {
///     conn.start_transaction()
///         .and_then(|transaction| {
///             MySelect::query_with_transaction(transaction, &A, &44)
///         })
///         .and_then(|(transation, read_result)| {
///             MyInsert::query_with_transaction(transation, &[(&2,)])
///         })
///         .and_then(|(transaction, write_result)| {
///             transaction.commit()
///         })
/// }
/// #
/// # fn main() {}
/// ```
pub enum Transaction {
    /// It is important to know that when creating a transaction with Sqlite any next attempt at
    /// creating a transaction will wait until the previous transaction has been completed. This
    /// means that the caller might introduce a deadlock if they are not careful.
    ///
    /// When a Sqlite transaction is dropped a "rollback" is performed, so one should always make
    /// sure to call "commit" if they want to persist the transation.
    Sqlite(Option<SqliteConnectionGuard>),
    /// An enum variant for the mysql-based transactions, your structure have to
    /// implement [crate::deprecated_mysql::MysqlTransaction] in order to be usable here.
    ///
    /// This backend is based on MyRouter connections and is deprecated soon. Please
    /// use new Mysql client instead.
    DeprecatedMysql(Option<BoxMysqlTransaction>),
    /// A variant used for the new Mysql client connection.
    Mysql(Option<mysql::Transaction>),
}

impl Transaction {
    /// Create a new transaction for the provided connection using default
    /// transaction options.
    pub fn new(connection: &super::Connection) -> BoxFuture<Transaction, Error> {
        Transaction::new_with_options(connection, TransactionOptions::new())
    }

    /// Create a new transaction for the provided connection using provided
    /// transaction options.
    pub fn new_with_options(
        connection: &super::Connection,
        options: TransactionOptions,
    ) -> BoxFuture<Transaction, Error> {
        match connection {
            super::Connection::Sqlite(con) => {
                let con = con.get_sqlite_guard();
                // Transactions in SQLite are always SERIALIZABLE; no transaction options.
                con.execute_batch("BEGIN DEFERRED")
                    .map(move |_| Transaction::Sqlite(Some(con)))
                    .into_future()
                    .map_err(failure_ext::convert)
                    .boxify()
            }
            super::Connection::DeprecatedMysql(con) => con
                .transaction_with_options(options)
                .map(|con| Transaction::DeprecatedMysql(Some(con)))
                .boxify(),
            super::Connection::Mysql(conn) => {
                cloned!(conn);
                async move {
                    let transaction = conn.begin_transaction().map_err(Error::from).await?;
                    Ok(Transaction::Mysql(Some(transaction)))
                }
                .boxed()
                .compat()
                .boxify()
            }
        }
    }

    /// Perform a commit on this transaction
    pub fn commit(mut self) -> BoxFuture<(), Error> {
        match self {
            Transaction::Sqlite(ref mut con) => {
                let actual_con = con.take().unwrap();
                let res = match actual_con.execute_batch("COMMIT") {
                    // Successfully committed, need to give the connection back
                    Ok(()) => Ok(()),
                    // Put it back so rollback will be performed on drop
                    err @ Err(_) => {
                        *con = Some(actual_con);
                        err
                    }
                };

                res.into_future().map_err(failure_ext::convert).boxify()
            }
            Transaction::DeprecatedMysql(ref mut con) => {
                con.take().expect("Called commit after drop").commit()
            }
            Transaction::Mysql(ref mut tr) => {
                let tr = tr.take().expect("Called commit after drop");
                async move { tr.commit().await }
                    .map_err(Error::from)
                    .boxed()
                    .compat()
                    .boxify()
            }
        }
    }

    /// Perform a rollback on this transaction
    pub fn rollback(mut self) -> BoxFuture<(), Error> {
        match self {
            Transaction::DeprecatedMysql(ref mut con) => {
                con.take().expect("Called rollback after drop").rollback()
            }
            // Sqlite will rollback on drop
            Transaction::Sqlite(..) => Ok(()).into_future().boxify(),
            Transaction::Mysql(ref mut tr) => {
                let tr = tr.take().expect("Called rollback after drop");
                async move { tr.rollback().await }
                    .map_err(Error::from)
                    .boxed()
                    .compat()
                    .boxify()
            }
        }
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        match self {
            Transaction::Sqlite(ref mut con) => {
                let con = if let Some(con) = con {
                    con
                } else {
                    // Transaction was already rollbacked or committed manually
                    return;
                };

                if let Err(err) = con.execute_batch("ROLLBACK") {
                    panic!(
                        "Rollback on drop of Sqlite connection has failed: {:#?}",
                        err
                    );
                }
            }
            Transaction::DeprecatedMysql(_) => {}
            Transaction::Mysql(_) => {}
        }
    }
}
