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

use crate::Binary;
use crate::Connection;
use crate::mysql::AsSql;
use crate::rusqlite::Connection as SqliteConnection;

fn binary_inputs() -> Vec<Vec<u8>> {
    vec![
        vec![],
        b"plain text".to_vec(),
        "café".as_bytes().to_vec(),
        b"\0'\"\\\n\r\x1a".to_vec(),
        (0..=255).collect(),
    ]
}

#[test]
fn test_binary_literal_sqlite() {
    let conn = SqliteConnection::open_in_memory().unwrap();
    for bytes in binary_inputs() {
        for no_backslash_escape in [false, true] {
            let literal = Binary(&bytes).as_sql(no_backslash_escape);
            let (value, kind): (Vec<u8>, String) = conn
                .query_row(&format!("SELECT {literal}, typeof({literal})"), [], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .unwrap();
            assert_eq!(value, bytes);
            assert_eq!(kind, "blob");
        }
    }
}

#[tokio::test]
#[allow(unused_imports)]
async fn test_binary_parameters_sqlite() {
    crate::queries! {
        write InsertBinary(values: (id: i64, value: Binary<'_>)) {
            none,
            "INSERT INTO binary_values (id, value) VALUES {values}"
        }
        write UpdateBinary(value: Binary<'_>, >list ids: i64) {
            none,
            "UPDATE binary_values SET value = {value} WHERE id IN {ids}"
        }
        read ReadBinary(value: Binary<'_>) -> (i64, Vec<u8>, String) {
            "SELECT id, value, typeof(value) FROM binary_values WHERE value = {value} ORDER BY id"
        }
        read ReadBinaryList(>list values: Binary<'_>) -> (i64) {
            "SELECT id FROM binary_values WHERE value IN {values} ORDER BY id"
        }
    }

    let conn = SqliteConnection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE binary_values (id INTEGER PRIMARY KEY, value BLOB)")
        .unwrap();
    let conn = Connection::with_sqlite(conn);
    let inputs = binary_inputs();
    let values: Vec<_> = inputs.iter().map(|bytes| Binary(bytes)).collect();
    let ids: Vec<_> = (0..values.len() as i64).collect();
    let rows: Vec<_> = ids.iter().zip(&values).collect();

    InsertBinary::query(&conn, &rows).await.unwrap();
    for (id, value) in &rows {
        assert_eq!(
            ReadBinary::query(&conn, value).await.unwrap(),
            vec![(**id, value.0.to_vec(), "blob".to_owned())],
        );
    }
    assert_eq!(
        ReadBinaryList::query(&conn, &values).await.unwrap(),
        ids.iter().map(|id| (*id,)).collect::<Vec<_>>(),
    );

    let replacement = Binary(b"replacement\0");
    let transaction = conn.start_transaction().await.unwrap();
    let (transaction, _) = UpdateBinary::query_with_transaction(transaction, &replacement, &ids)
        .await
        .unwrap();
    let (transaction, rows) = ReadBinary::query_with_transaction(transaction, &replacement)
        .await
        .unwrap();
    assert_eq!(rows.len(), values.len());
    transaction.rollback().await.unwrap();
    assert_eq!(
        ReadBinary::query(&conn, &replacement).await.unwrap(),
        vec![],
    );

    let transaction = conn.start_transaction().await.unwrap();
    let (transaction, _) =
        InsertBinary::query_with_transaction(transaction, &[(&10, &replacement)])
            .await
            .unwrap();
    transaction.commit().await.unwrap();
    assert_eq!(
        ReadBinary::query(&conn, &replacement).await.unwrap(),
        vec![(10, replacement.0.to_vec(), "blob".to_owned())],
    );
}

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

#[cfg(shed_sql_mysql)]
#[cfg(test)]
mod mysql {

    use anyhow::Result;
    use fbinit::FacebookInit;
    use mysql_client::InstanceRequirement;
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
            Some(
                "CREATE TABLE IF NOT EXISTS foo(
                x INT,
                y DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                test CHAR(64),
                id INT AUTO_INCREMENT,
                PRIMARY KEY(id)
            )",
            ),
            InstanceRequirement::Master,
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
