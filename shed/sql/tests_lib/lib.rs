/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, clippy::all)]

use chrono::NaiveDate;
use chrono::NaiveDateTime;
use rand::distributions::Alphanumeric;
use rand::thread_rng;
use rand::Rng;
use sql::anyhow::Error;
use sql::mysql_async::prelude::*;
use sql::mysql_async::FromValueError;
use sql::mysql_async::Value;
use sql::queries;
use sql::sql_common::mysql;
use sql::Connection;
use sql::Transaction;

pub struct A;

impl ToValue for A {
    fn to_value(&self) -> Value {
        Value::NULL
    }
}

#[derive(Debug, Eq, PartialEq, mysql::TryFromRowField)]
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
    write TestQuery11(x: i64, test: String) {
        none,
        "INSERT INTO foo (x, test) VALUES ({x}, {test})"
    }
    read TestQuery12(test: String) -> (i64) {
        "SELECT x FROM foo WHERE test = {test}"
    }

    write TestQuery13(x: i64, y: NaiveDateTime) {
        none,
        "INSERT INTO foo (x, y) VALUES ({x}, {y})"
    }

    read TestQuery14(date: NaiveDateTime) -> (String) {
        "SELECT datetime(y) FROM foo WHERE y = {date}"
    }
}

pub async fn test_basic_query(conn: Connection) -> Result<(), Error> {
    let rng = thread_rng();
    let test: String = rng
        .sample_iter(Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    TestQuery11::query(&conn, &1, &test).await?;
    let res = TestQuery11::query(&conn, &3, &test).await?;
    assert_eq!(res.affected_rows(), 1);

    let res = TestQuery12::query(&conn, &test).await?;
    assert_eq!(res, vec![(1,), (3,)]);
    Ok(())
}

pub async fn test_basic_transaction(conn: Connection) {
    let rng = thread_rng();
    let test: String = rng
        .sample_iter(Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let transaction = conn.start_transaction().await.unwrap();
    let (transaction, _res) = TestQuery11::query_with_transaction(transaction, &5, &test)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let res = TestQuery12::query(&conn, &test).await.unwrap();
    assert_eq!(res, vec![(5,)]);
}

pub async fn test_read_query(conn: Connection, semantics: TestSemantics) {
    assert_eq!(
        TestQuery::query(&conn, &A, &72).await.unwrap(),
        vec![(44, B, B, 72)]
    );
    assert_eq!(TestQuery2::query(&conn).await.unwrap(), vec![(44, B)]);
    assert_eq!(
        TestQuery6::query(&conn).await.unwrap(),
        vec![(match semantics {
            TestSemantics::Mysql => 6i64,
            TestSemantics::Sqlite => 7i64,
        },)]
    );
}

pub async fn test_datetime_query(conn: Connection) {
    let date = NaiveDate::from_ymd(2021, 1, 21).and_hms(21, 21, 21);
    let res = TestQuery13::query(&conn, &3, &date).await.unwrap();
    assert_eq!(res.affected_rows(), 1);

    let res = TestQuery14::query(&conn, &date).await.unwrap();
    assert_eq!(res, vec![("2021-01-21 21:21:21".to_owned(),)]);
}

pub async fn test_write_query(conn: Connection) {
    let res = TestQuery3::query(&conn, &[(&44,)]).await.unwrap();
    assert_eq!(res.affected_rows(), 1);
    assert_eq!(res.last_insert_id(), Some(1));

    let res = TestQuery3::query(&conn, &[(&72,), (&53,)]).await.unwrap();
    assert_eq!(res.affected_rows(), 2);
    assert_eq!(res.last_insert_id(), Some(3));

    assert_eq!(
        TestQuery4::query(&conn, &1, &3).await.unwrap(),
        vec![(44,), (72,), (53,)]
    );

    let res = TestQuery7::query(&conn, &123).await.unwrap();
    assert_eq!(res.affected_rows(), 1);
    assert_eq!(res.last_insert_id(), Some(1));

    assert_eq!(
        TestQuery5::query(&conn, &[1, 2, 3]).await.unwrap(),
        vec![(123,), (72,), (53,)]
    );

    let res = TestQuery8::query(&conn, &[1, 2]).await.unwrap();
    assert_eq!(res.affected_rows(), 2);

    assert_eq!(
        TestQuery5::query(&conn, &[1, 2, 3]).await.unwrap(),
        vec![(456,), (456,), (53,)]
    );

    let res = TestQuery9::query(&conn, &123, &[1, 2]).await.unwrap();
    assert_eq!(res.affected_rows(), 2);

    assert_eq!(
        TestQuery5::query(&conn, &[1, 2, 3]).await.unwrap(),
        vec![(123,), (123,), (53,)]
    );

    let res = TestQuery10::query(&conn, &456, &3).await.unwrap();
    assert_eq!(res.affected_rows(), 1);

    assert_eq!(
        TestQuery5::query(&conn, &[1, 2, 3]).await.unwrap(),
        vec![(123,), (123,), (456,)]
    );
}

pub enum TestSemantics {
    Sqlite,
    Mysql,
}

pub async fn in_transaction(transaction: Transaction, semantics: TestSemantics) -> Transaction {
    let (transaction, res) = TestQuery3::query_with_transaction(transaction, &[(&44,)])
        .await
        .unwrap();
    assert_eq!(res.affected_rows(), 1);
    assert_eq!(res.last_insert_id(), Some(1));

    let (transaction, res) = TestQuery3::query_with_transaction(transaction, &[(&72,), (&53,)])
        .await
        .unwrap();
    assert_eq!(res.affected_rows(), 2);
    match semantics {
        // MySQL returns first ID for multi-row inserts
        TestSemantics::Mysql => assert_eq!(res.last_insert_id(), Some(2)),
        TestSemantics::Sqlite => assert_eq!(res.last_insert_id(), Some(3)),
    }

    let (transaction, res) = TestQuery4::query_with_transaction(transaction, &1, &3)
        .await
        .unwrap();
    assert_eq!(res, vec![(44,), (72,), (53,)]);

    let (transaction, res) = TestQuery7::query_with_transaction(transaction, &123)
        .await
        .unwrap();
    match semantics {
        // MySQL counts a replace of an existing row as affecting two rows.
        TestSemantics::Mysql => assert_eq!(res.affected_rows(), 2),
        TestSemantics::Sqlite => assert_eq!(res.affected_rows(), 1),
    }
    assert_eq!(res.last_insert_id(), Some(1));

    let (transaction, res) = TestQuery5::query_with_transaction(transaction, &[1, 2, 3])
        .await
        .unwrap();
    assert_eq!(res, vec![(123,), (72,), (53,)]);

    transaction
}

pub async fn test_transaction_rollback(conn: Connection, semantics: TestSemantics) {
    let transaction = conn.start_transaction().await.unwrap();
    let transaction = in_transaction(transaction, semantics).await;
    transaction.rollback().await.unwrap();

    assert_eq!(TestQuery4::query(&conn, &1, &3).await.unwrap(), vec![]);
}

pub async fn test_transaction_rollback_on_drop(conn: Connection, semantics: TestSemantics) {
    let transaction = conn.start_transaction().await.unwrap();
    // dropping transaction here should trigger rollback
    let _ = in_transaction(transaction, semantics);

    assert_eq!(TestQuery4::query(&conn, &1, &3).await.unwrap(), vec![]);
}

pub async fn test_transaction_commit(conn: Connection, semantics: TestSemantics) {
    let transaction = conn.start_transaction().await.unwrap();
    let transaction = in_transaction(transaction, semantics).await;
    transaction.commit().await.unwrap();

    assert_eq!(
        TestQuery4::query(&conn, &1, &3).await.unwrap(),
        vec![(123,), (72,), (53,)]
    );
}
