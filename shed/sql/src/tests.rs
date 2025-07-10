/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

#![deny(warnings)]

use sql_tests_lib::TestSemantics;
use sql_tests_lib::test_datetime_query;
use sql_tests_lib::test_query_visibility_modifiers_compile;
use sql_tests_lib::test_read_query;
use sql_tests_lib::test_transaction_commit;
use sql_tests_lib::test_transaction_rollback;
use sql_tests_lib::test_transaction_rollback_on_drop;
use sql_tests_lib::test_write_query;

use crate::Connection;
use crate::rusqlite::Connection as SqliteConnection;

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

#[tokio::test]
async fn test_visibility_modifiers_compile_with_sqlite() {
    test_query_visibility_modifiers_compile(prepare_sqlite_con()).await;
}

#[cfg(fbcode_build)]
#[cfg(test)]
mod mysql {

    use anyhow::Result;
    use fbinit::FacebookInit;
    use sql_tests_lib::mysql_test_lib::setup_mysql_test_connection;
    use sql_tests_lib::mysql_test_lib::test_basic_read_query_telemetry;
    use sql_tests_lib::mysql_test_lib::test_basic_write_query_telemetry;
    use sql_tests_lib::mysql_test_lib::test_transaction_read_query_telemetry;
    use sql_tests_lib::mysql_test_lib::test_transaction_write_query_telemetry;
    use sql_tests_lib::test_basic_query;
    use sql_tests_lib::test_basic_transaction;

    use super::*;

    async fn setup_connection(fb: FacebookInit) -> Result<Connection> {
        setup_mysql_test_connection(
            fb,
            "CREATE TABLE IF NOT EXISTS foo(
                x INT,
                y DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                test CHAR(64),
                id INT AUTO_INCREMENT,
                PRIMARY KEY(id)
            )",
        )
        .await
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

    #[fbinit::test]
    async fn test_mysql_basic_read_query_telemetry(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_basic_read_query_telemetry(conn).await?;
        Ok(())
    }

    #[fbinit::test]
    async fn test_mysql_transaction_read_query_telemetry(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_transaction_read_query_telemetry(conn).await?;
        Ok(())
    }
    #[fbinit::test]
    async fn test_mysql_basic_write_query_telemetry(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_basic_write_query_telemetry(conn).await?;
        Ok(())
    }

    #[fbinit::test]
    async fn test_mysql_transaction_write_query_telemetry(fb: FacebookInit) -> Result<()> {
        let conn = setup_connection(fb).await?;
        test_transaction_write_query_telemetry(conn).await?;
        Ok(())
    }
}
