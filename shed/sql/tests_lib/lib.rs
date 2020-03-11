/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![deny(warnings, clippy::all)]

use futures::Future;
use sql::mysql_async::prelude::*;
use sql::mysql_async::{FromValueError, Value};
use sql::{queries, Connection, Transaction};

pub struct A;

impl ToValue for A {
    fn to_value(&self) -> Value {
        Value::NULL
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct B;
pub struct IntB;

impl ConvIr<B> for IntB {
    fn new(v: Value) -> Result<Self, FromValueError> {
        match v {
            Value::NULL => Ok(IntB),
            v => Err(FromValueError(v)),
        }
    }

    fn commit(self) -> B {
        B
    }

    fn rollback(self) -> Value {
        Value::NULL
    }
}

impl FromValue for B {
    type Intermediate = IntB;
}

queries! {
    read TestQuery(param_a: A, param_uint: u64) -> (u64, B, B, i64) {
        "SELECT 44, NULL, {param_a}, {param_uint}"
    }
    read TestQuery2() -> (u64, B) {
        "SELECT 44, NULL"
    }
    write TestQuery3(values: (x: i64)) {
        none,
        "INSERT INTO foo (x) VALUES {values}"
    }
    read TestQuery4(id1: u64, id2: u64) -> (i64) {
        "SELECT x FROM foo WHERE {id1} <= ID and ID <= {id2}"
    }
    read TestQuery5(>list id: u64) -> (i64) {
        "SELECT x FROM foo WHERE ID IN {id}"
    }
    read TestQuery6() -> (i64) {
        mysql("SELECT 6")
        sqlite("SELECT 7")
    }
    write TestQuery7(x: i64) {
        none,
        "REPLACE INTO foo (ID, x) VALUES (1, {x})"
    }
    write TestQuery8(>list ids: u64) {
        none,
        "UPDATE foo SET x = 456 WHERE id IN {ids}"
    }
    write TestQuery9(x: u64, >list ids: u64) {
        none,
        "UPDATE foo SET x = {x} WHERE id IN {ids}"
    }
    write TestQuery10(x: u64, id: u64) {
        none,
        "UPDATE foo SET x = {x} WHERE id = {id}"
    }
}

pub fn test_read_query(conn: Connection, semantics: TestSemantics) {
    assert_eq!(
        TestQuery::query(&conn, &A, &72).wait().unwrap(),
        vec![(44, B, B, 72)]
    );
    assert_eq!(TestQuery2::query(&conn).wait().unwrap(), vec![(44, B)]);
    assert_eq!(
        TestQuery6::query(&conn).wait().unwrap(),
        vec![(match semantics {
            TestSemantics::Mysql => 6i64,
            TestSemantics::Sqlite => 7i64,
        },)]
    );
}

pub fn test_write_query(conn: Connection) {
    let res = TestQuery3::query(&conn, &[(&44,)]).wait().unwrap();
    assert_eq!(res.affected_rows(), 1);
    assert_eq!(res.last_insert_id(), Some(1));

    let res = TestQuery3::query(&conn, &[(&72,), (&53,)]).wait().unwrap();
    assert_eq!(res.affected_rows(), 2);
    assert_eq!(res.last_insert_id(), Some(3));

    assert_eq!(
        TestQuery4::query(&conn, &1, &3).wait().unwrap(),
        vec![(44,), (72,), (53,)]
    );

    let res = TestQuery7::query(&conn, &123).wait().unwrap();
    assert_eq!(res.affected_rows(), 1);
    assert_eq!(res.last_insert_id(), Some(1));

    assert_eq!(
        TestQuery5::query(&conn, &[1, 2, 3]).wait().unwrap(),
        vec![(123,), (72,), (53,)]
    );

    let res = TestQuery8::query(&conn, &[1, 2]).wait().unwrap();
    assert_eq!(res.affected_rows(), 2);

    assert_eq!(
        TestQuery5::query(&conn, &[1, 2, 3]).wait().unwrap(),
        vec![(456,), (456,), (53,)]
    );

    let res = TestQuery9::query(&conn, &123, &[1, 2]).wait().unwrap();
    assert_eq!(res.affected_rows(), 2);

    assert_eq!(
        TestQuery5::query(&conn, &[1, 2, 3]).wait().unwrap(),
        vec![(123,), (123,), (53,)]
    );

    let res = TestQuery10::query(&conn, &456, &3).wait().unwrap();
    assert_eq!(res.affected_rows(), 1);

    assert_eq!(
        TestQuery5::query(&conn, &[1, 2, 3]).wait().unwrap(),
        vec![(123,), (123,), (456,)]
    );
}

pub enum TestSemantics {
    Sqlite,
    Mysql,
}

pub fn in_transaction(transaction: Transaction, semantics: TestSemantics) -> Transaction {
    let (transaction, res) = TestQuery3::query_with_transaction(transaction, &[(&44,)])
        .wait()
        .unwrap();
    assert_eq!(res.affected_rows(), 1);
    assert_eq!(res.last_insert_id(), Some(1));

    let (transaction, res) = TestQuery3::query_with_transaction(transaction, &[(&72,), (&53,)])
        .wait()
        .unwrap();
    assert_eq!(res.affected_rows(), 2);
    match semantics {
        // MySQL returns first ID for multi-row inserts
        TestSemantics::Mysql => assert_eq!(res.last_insert_id(), Some(2)),
        TestSemantics::Sqlite => assert_eq!(res.last_insert_id(), Some(3)),
    }

    let (transaction, res) = TestQuery4::query_with_transaction(transaction, &1, &3)
        .wait()
        .unwrap();
    assert_eq!(res, vec![(44,), (72,), (53,)]);

    let (transaction, res) = TestQuery7::query_with_transaction(transaction, &123)
        .wait()
        .unwrap();
    match semantics {
        // MySQL counts a replace of an existing row as affecting two rows.
        TestSemantics::Mysql => assert_eq!(res.affected_rows(), 2),
        TestSemantics::Sqlite => assert_eq!(res.affected_rows(), 1),
    }
    assert_eq!(res.last_insert_id(), Some(1));

    let (transaction, res) = TestQuery5::query_with_transaction(transaction, &[1, 2, 3])
        .wait()
        .unwrap();
    assert_eq!(res, vec![(123,), (72,), (53,)]);

    transaction
}

pub fn test_transaction_rollback(conn: Connection, semantics: TestSemantics) {
    let transaction = conn.start_transaction().wait().unwrap();
    let transaction = in_transaction(transaction, semantics);
    transaction.rollback().wait().unwrap();

    assert_eq!(TestQuery4::query(&conn, &1, &3).wait().unwrap(), vec![]);
}

pub fn test_transaction_rollback_on_drop(conn: Connection, semantics: TestSemantics) {
    let transaction = conn.start_transaction().wait().unwrap();
    // dropping transaction here should trigger rollback
    let _ = in_transaction(transaction, semantics);

    assert_eq!(TestQuery4::query(&conn, &1, &3).wait().unwrap(), vec![]);
}

pub fn test_transaction_commit(conn: Connection, semantics: TestSemantics) {
    let transaction = conn.start_transaction().wait().unwrap();
    let transaction = in_transaction(transaction, semantics);
    transaction.commit().wait().unwrap();

    assert_eq!(
        TestQuery4::query(&conn, &1, &3).wait().unwrap(),
        vec![(123,), (72,), (53,)]
    );
}
