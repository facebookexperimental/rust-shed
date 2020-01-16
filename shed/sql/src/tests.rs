/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
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
