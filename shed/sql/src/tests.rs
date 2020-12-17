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
        ConnectionPoolOptionsBuilder, DbLocator, InstanceRequirement, ShardableConnectionPool,
        ShardableMysqlCppClient,
    };
    use sql_tests_lib::{test_basic_query, test_basic_transaction};

    async fn setup_connection(fb: FacebookInit) -> Result<Connection> {
        let locator = DbLocator::new("xdb.dbclient_test.1", InstanceRequirement::Master)?;
        let client = ShardableMysqlCppClient::new(fb)?;

        client
            .query_raw(
                &locator,
                "CREATE TABLE IF NOT EXISTS foo(x INT, test CHAR(64), id INT AUTO_INCREMENT, PRIMARY KEY(id))",
            )
            .await?;

        let pool_options = ConnectionPoolOptionsBuilder::default()
            .pool_limit(1)
            .build()
            .map_err(Error::msg)?;
        let pool = ShardableConnectionPool::new(&client, &pool_options)?.bind(locator);

        let conn = MysqlConnection::new(pool);
        Ok(Connection::from(conn))
    }

    #[fbinit::test]
    async fn test_mysql_basic_query(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_basic_query(conn)
    }

    #[fbinit::test]
    async fn test_mysql_transaction(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_basic_transaction(conn);
        Ok(())
    }
}
