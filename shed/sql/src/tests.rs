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
