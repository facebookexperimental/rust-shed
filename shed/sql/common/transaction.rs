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
use futures::compat::Future01CompatExt;
use futures::future::TryFutureExt;
use mysql_async::TransactionOptions;

use crate::deprecated_mysql::BoxMysqlTransaction;
use crate::mysql;
use crate::sqlite::SqliteConnectionGuard;

impl crate::Connection {
    /// Start an SQL transaction for this connection. Refer to `transaction::Transaction` docs for
    /// more info
    pub async fn start_transaction(&self) -> Result<Transaction, Error> {
        Transaction::new(self).await
    }

    /// Start an SQL transaction for this connection. Refer to `transaction::Transaction` docs for
    /// more info
    pub async fn start_transaction_with_options(
        &self,
        options: TransactionOptions,
    ) -> Result<Transaction, Error> {
        Transaction::new_with_options(self, options).await
    }
}

/// Enum for generalizing transactions over Sqlite and MyRouter.
///
/// # Example
/// ```
/// use anyhow::Error;
/// use futures::Future;
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
/// async fn foo(conn: Connection) -> Result<(), Error> {
///     let transaction = conn.start_transaction().await?;
///     let (transaction, read_result) =
///         MySelect::query_with_transaction(transaction, &A, &44).await?;
///     let (transaction, write_result) =
///         MyInsert::query_with_transaction(transaction, &[(&2,)]).await?;
///     transaction.commit().await
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
    pub async fn new(connection: &super::Connection) -> Result<Transaction, Error> {
        Transaction::new_with_options(connection, TransactionOptions::new()).await
    }

    /// Create a new transaction for the provided connection using provided
    /// transaction options.
    pub async fn new_with_options(
        connection: &super::Connection,
        options: TransactionOptions,
    ) -> Result<Transaction, Error> {
        match connection {
            super::Connection::Sqlite(con) => {
                let con = con.get_sqlite_guard();
                // Transactions in SQLite are always SERIALIZABLE; no transaction options.
                con.execute_batch("BEGIN DEFERRED")
                    .map(move |_| Transaction::Sqlite(Some(con)))
                    .map_err(failure_ext::convert)
            }
            super::Connection::DeprecatedMysql(con) => {
                let transaction = con.transaction_with_options(options).compat().await?;
                Ok(Transaction::DeprecatedMysql(Some(transaction)))
            }
            super::Connection::Mysql(conn) => {
                let transaction = conn.begin_transaction().map_err(Error::from).await?;
                Ok(Transaction::Mysql(Some(transaction)))
            }
        }
    }

    /// Perform a commit on this transaction
    pub async fn commit(mut self) -> Result<(), Error> {
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

                Ok(res?)
            }
            Transaction::DeprecatedMysql(ref mut con) => {
                con.take()
                    .expect("Called commit after drop")
                    .commit()
                    .compat()
                    .await
            }
            Transaction::Mysql(ref mut tr) => {
                let tr = tr.take().expect("Called commit after drop");
                Ok(tr.commit().await?)
            }
        }
    }

    /// Perform a rollback on this transaction
    pub async fn rollback(mut self) -> Result<(), Error> {
        match self {
            Transaction::DeprecatedMysql(ref mut con) => {
                con.take()
                    .expect("Called rollback after drop")
                    .rollback()
                    .compat()
                    .await
            }
            // Sqlite will rollback on drop
            Transaction::Sqlite(..) => Ok(()),
            Transaction::Mysql(ref mut tr) => {
                let tr = tr.take().expect("Called rollback after drop");
                Ok(tr.rollback().await?)
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
