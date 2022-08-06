/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings)]

use sql_tests_lib::test_datetime_query;
use sql_tests_lib::test_read_query;
use sql_tests_lib::test_transaction_commit;
use sql_tests_lib::test_transaction_rollback;
use sql_tests_lib::test_transaction_rollback_on_drop;
use sql_tests_lib::test_write_query;
use sql_tests_lib::TestSemantics;

use crate::rusqlite::Connection as SqliteConnection;
use crate::Connection;

#[tokio::test]
async fn test_read_query_sqlite() {
    test_read_query(
        Connection::with_sqlite(SqliteConnection::open_in_memory().unwrap()),
        TestSemantics::Sqlite,
    )
    .await
}

fn prepare_sqlite_con() -> Connection {
    let conn = SqliteConnection::open_in_memory().unwrap();
    conn.execute_batch(
        "BEGIN;
            CREATE TABLE foo(
                x INTEGER,
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                y DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            COMMIT;",
    )
    .unwrap();
    Connection::with_sqlite(conn)
}

#[tokio::test]
async fn test_datetime_query_with_sqlite() {
    test_datetime_query(prepare_sqlite_con()).await;
}

#[tokio::test]
async fn test_write_query_with_sqlite() {
    test_write_query(prepare_sqlite_con()).await;
}

#[tokio::test]
async fn test_transaction_rollback_with_sqlite() {
    test_transaction_rollback(prepare_sqlite_con(), TestSemantics::Sqlite).await;
}

#[tokio::test]
async fn test_transaction_rollback_on_drop_with_sqlite() {
    test_transaction_rollback_on_drop(prepare_sqlite_con(), TestSemantics::Sqlite).await;
}

#[tokio::test]
async fn test_transaction_commit_with_sqlite() {
    test_transaction_commit(prepare_sqlite_con(), TestSemantics::Sqlite).await;
}

#[cfg(fbcode_build)]
#[cfg(test)]
mod mysql {
    use std::sync::Arc;

    use anyhow::Error;
    use anyhow::Result;
    use fbinit::FacebookInit;
    use mysql_client::ConnectionPool;
    use mysql_client::ConnectionPoolOptionsBuilder;
    use mysql_client::DbLocator;
    use mysql_client::InstanceRequirement;
    use mysql_client::MysqlCppClient;
    use sql_tests_lib::test_basic_query;
    use sql_tests_lib::test_basic_transaction;

    use super::*;
    use crate::sql_common::mysql::Connection as MysqlConnection;
    use crate::sql_common::mysql::ConnectionStats;

    async fn setup_connection(fb: FacebookInit) -> Result<Connection> {
        let locator = DbLocator::new("xdb.dbclient_test.1", InstanceRequirement::Master)?;
        let client = MysqlCppClient::new(fb)?;

        client
            .query_raw(
                &locator,
                "CREATE TABLE IF NOT EXISTS foo(
                    x INT,
                    y DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                    test CHAR(64),
                    id INT AUTO_INCREMENT,
                    PRIMARY KEY(id)
                )",
            )
            .await?;

        let pool_options = ConnectionPoolOptionsBuilder::default()
            .pool_limit(1)
            .build()
            .map_err(Error::msg)?;
        let pool = ConnectionPool::new(&client, &pool_options)?.bind(locator);

        let stats = Arc::new(ConnectionStats::new("test".to_string()));
        let conn = MysqlConnection::new(pool, stats);
        Ok(Connection::from(conn))
    }

    #[fbinit::test]
    async fn test_mysql_basic_query(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_basic_query(conn).await?;
        Ok(())
    }

    #[fbinit::test]
    async fn test_mysql_transaction(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_basic_transaction(conn).await;
        Ok(())
    }
}
