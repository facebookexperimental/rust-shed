/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings)]

use sql_tests_lib::{
    test_read_query, test_transaction_commit, test_transaction_rollback,
    test_transaction_rollback_on_drop, test_write_query, TestSemantics,
};

use crate::rusqlite::Connection as SqliteConnection;
use crate::Connection;

#[test]
fn test_read_query_sqlite() {
    test_read_query(
        Connection::with_sqlite(SqliteConnection::open_in_memory().unwrap()),
        TestSemantics::Sqlite,
    );
}

fn prepare_sqlite_con() -> Connection {
    let conn = SqliteConnection::open_in_memory().unwrap();
    conn.execute_batch(
        "BEGIN;
            CREATE TABLE foo(x INTEGER, id INTEGER PRIMARY KEY AUTOINCREMENT);
            COMMIT;",
    )
    .unwrap();
    Connection::with_sqlite(conn)
}

#[test]
fn test_write_query_with_sqlite() {
    test_write_query(prepare_sqlite_con());
}

#[test]
fn test_transaction_rollback_with_sqlite() {
    test_transaction_rollback(prepare_sqlite_con(), TestSemantics::Sqlite);
}

#[test]
fn test_transaction_rollback_on_drop_with_sqlite() {
    test_transaction_rollback_on_drop(prepare_sqlite_con(), TestSemantics::Sqlite);
}

#[test]
fn test_transaction_commit_with_sqlite() {
    test_transaction_commit(prepare_sqlite_con(), TestSemantics::Sqlite);
}

#[cfg(fbcode_build)]
#[cfg(test)]
mod mysql {
    use super::*;
    use crate::sql_common::mysql::Connection as MysqlConnection;

    use anyhow::{Error, Result};
    use fbinit::FacebookInit;
    use mysql_client::{
        ConnectionPool, ConnectionPoolOptionsBuilder, DbLocator, InstanceRequirement,
        MysqlCppClient,
    };
    use sql_tests_lib::{test_basic_query, test_basic_transaction};

    async fn setup_connection(fb: FacebookInit) -> Result<Connection> {
        let locator = DbLocator::new("xdb.dbclient_test.1", InstanceRequirement::Master)?;
        let mut client = MysqlCppClient::new(fb, locator)?;

        client
            .query_raw(
                "CREATE TABLE IF NOT EXISTS foo(x INT, test CHAR(64), id INT AUTO_INCREMENT, PRIMARY KEY(id))",
            )
            .await?;

        let pool_options = ConnectionPoolOptionsBuilder::default()
            .pool_limit(1)
            .build()
            .map_err(Error::msg)?;
        let pool = ConnectionPool::new(&mut client, &pool_options)?;

        let conn = MysqlConnection::new(pool);
        Ok(Connection::from(conn))
    }

    #[fbinit::test]
    async fn test_mysql2_basic_query(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_basic_query(conn)
    }

    #[fbinit::test]
    async fn test_mysql2_transaction(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_basic_transaction(conn);
        Ok(())
    }
}

mod sqlite {
    use super::*;
    use crate::queries;
    use futures::channel::oneshot;
    use futures_util::compat::Future01CompatExt;

    queries! {
        write TestInsert(values: (x: i64)) {
            none,
            "INSERT INTO foo (x) VALUES {values}"
        }
    }

    async fn query_with_suspended_transaction(
        conn: Connection,
        notify_transaction_started: oneshot::Sender<()>,
        transaction_unblocked: oneshot::Receiver<()>,
    ) {
        let transaction = conn
            .start_transaction()
            .compat()
            .await
            .expect("Failed to start transaction");
        notify_transaction_started
            .send(())
            .expect("Transaction started - got global lock");
        transaction_unblocked
            .await
            .expect("Failed to unblock transaction");
        let (transaction, _res) = TestInsert::query_with_transaction(transaction, &[(&42,)])
            .compat()
            .await
            .expect("TestInsert query failed");
        transaction
            .commit()
            .compat()
            .await
            .expect("Failed to commit transaction");
    }

    #[tokio::test]
    async fn test_concurrent_transaction() {
        let conn = prepare_sqlite_con();

        let (notify_unblock_transaction_1, transaction_unblocked_1) = oneshot::channel();
        let (notify_transaction_started_1, transaction_started_1) = oneshot::channel();
        let query_1 = tokio::task::spawn(query_with_suspended_transaction(
            conn.clone(),
            notify_transaction_started_1,
            transaction_unblocked_1,
        ));
        transaction_started_1
            .await
            .expect("Failed to start a transaction 1");

        let (notify_unblock_transaction_2, transaction_unblocked_2) = oneshot::channel();
        let (notify_transaction_started_2, transaction_started_2) = oneshot::channel();
        let query_2 = tokio::task::spawn(query_with_suspended_transaction(
            conn.clone(),
            notify_transaction_started_2,
            transaction_unblocked_2,
        ));
        tokio::task::yield_now().await;

        notify_unblock_transaction_1
            .send(())
            .expect("Unlbocking query #1");
        query_1.await.expect("query-task #1 crashed");

        transaction_started_2
            .await
            .expect("Failed to start a transaction 2");

        notify_unblock_transaction_2
            .send(())
            .expect("Unlbocking query #2");
        query_2.await.expect("query-task #2 crashed");
    }
}
