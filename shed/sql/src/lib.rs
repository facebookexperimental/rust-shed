/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Crate for accessing Sql databases. Check the [Connection] enum for supported
//! types of databases.
//!
//! This crate API is heavily based on mysql_async. If you wish to use your custom structure in SQL
//! queries or to parse your structure from a result of SQL query then implement
//! `mysql_async::prelude::ToValue` and/or `mysql_async::prelude::::FromValue` for it.
//!
//! Queries are created using the `queries!` macro, you need to specify your query type to be either
//! `read` if you perform a SELECT and expect the result to be parsed into a tuple or `write` if
//! you execute an INSERT/UPDATE/DELETE query which will give you `WriteResult` upon completion.
//!
//! This crate also supports SQL transactions, see [Transaction] for more details.
//!
//! For some working example usage you can look at `tests.rs`, below is a simplified one.
//!
//! # Example
//! ```
//! use anyhow::Error;
//! use futures::Future;
//! use sql::Connection;
//! use sql::queries;
//! use sql_tests_lib::A;
//! use sql_tests_lib::B;
//!
//! queries! {
//!     read MySelect(param_a: A, param_uint: u64) -> (u64, B, B, i64) {
//!         "SELECT 44, NULL, {param_a}, {param_uint}"
//!     }
//!     write MyInsert(values: (x: i64)) {
//!         none,
//!         "INSERT INTO foo (x) VALUES ({values})"
//!     }
//! }
//!
//! async fn foo(conn: Connection) -> Result<(), Error> {
//!     assert_eq!(MySelect::query(&conn, &A, &72).await?, vec![(44, B, B, 72)]);
//!
//!     let res = MyInsert::query(&conn, &[(&44,)]).await?;
//!     assert_eq!(res.affected_rows(), 1);
//!     assert_eq!(res.last_insert_id(), Some(1));
//!     Ok(())
//! }
//! #
//! # fn main() {}
//! ```

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

#[cfg(test)]
mod tests;
pub use anyhow;
pub use cloned;
pub use frunk::HList;
pub use futures;
pub use futures_ext;
pub use futures_util;
pub use mysql_async;
use mysql_async::Value;
pub use rusqlite;
use rusqlite::Result as SqliteResult;
use rusqlite::types::FromSql as FromSqliteValue;
use rusqlite::types::FromSqlResult as FromSqliteValueResult;
use rusqlite::types::ToSql as ToSqliteValue;
use rusqlite::types::ToSqlOutput as ToSqliteOutput;
use rusqlite::types::Value as SqliteValue;
use rusqlite::types::ValueRef as SqliteValueRef;
pub use sql_common;
pub use sql_common::Connection;
pub use sql_common::QueryTelemetry;
pub use sql_common::SqlConnections;
pub use sql_common::SqlShardedConnections;
pub use sql_common::WriteResult;
pub use sql_common::mysql;
pub use sql_common::mysql::OssConnection;
pub use sql_common::sqlite;
pub use sql_common::transaction::Transaction;

/// Wrapper around MySql Value to implement Sqlite traits on it.
/// This should never be used directly, it is made public so that internal macros can make use of it
#[doc(hidden)]
pub struct ValueWrapper(pub Value);

impl ToSqliteValue for ValueWrapper {
    fn to_sql(&self) -> SqliteResult<ToSqliteOutput<'_>> {
        Ok(match &self.0 {
            Value::NULL => ToSqliteOutput::Owned(SqliteValue::Null),
            Value::Bytes(b) => ToSqliteOutput::Borrowed(SqliteValueRef::Blob(b.as_ref())),
            Value::Int(i) => ToSqliteOutput::Owned(SqliteValue::Integer(*i)),
            Value::UInt(u) => ToSqliteOutput::Owned(SqliteValue::Integer(*u as i64)),
            Value::Float(f) => ToSqliteOutput::Owned(SqliteValue::Real((*f).into())),
            Value::Double(f) => ToSqliteOutput::Owned(SqliteValue::Real(*f)),
            Value::Date(year, month, day, hour, min, sec, micro) => {
                ToSqliteOutput::Owned(SqliteValue::Text(format!(
                    "{year:04}-{month:02}-{day:02} {hour:02}:{min:02}:{sec:02}.{micro:06}"
                )))
            }
            Value::Time(..) => {
                unimplemented!("TODO(luk) implement time for sqlite")
            }
        })
    }
}

impl FromSqliteValue for ValueWrapper {
    fn column_result(value: SqliteValueRef<'_>) -> FromSqliteValueResult<Self> {
        Ok(ValueWrapper(match value {
            SqliteValueRef::Null => Value::NULL,
            SqliteValueRef::Integer(i) => Value::Int(i),
            SqliteValueRef::Real(f) => Value::Double(f),
            SqliteValueRef::Text(s) => Value::Bytes(s.into()),
            SqliteValueRef::Blob(b) => Value::Bytes(b.into()),
        }))
    }
}

#[macro_export]
/// TODO: write doc for this macro and consider rewriting this as a proc macro
macro_rules! queries {
    () => ();

    (
        $vi:vis read $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) -> ($( $rtype:ty ),* $(,)*) { $q:expr }
        $( $tt:tt )*
    ) => (
        $crate::queries! {
            $vi read $name (
                $( $pname: $ptype ),*
                $( >list $lname: $ltype )*
            ) -> ($( $rtype ),*) { mysql($q) sqlite($q) }
            $( $tt )*
        }
    );

    (
        $vi:vis read $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) -> ($( $rtype:ty ),* $(,)*) { mysql($mysql_q:expr) sqlite($sqlite_q:expr) }
        $( $tt:tt )*
    ) => (
        #[allow(non_snake_case)]
        $vi mod $name {
            $crate::_read_query_impl!((
                $( $pname: $ptype, )*
                $( >list $lname: $ltype )*
            ) -> ($( $rtype ),*) { mysql($mysql_q) sqlite($sqlite_q) });

            #[allow(dead_code)]
            pub async fn query(
                connection: & Connection,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> Result<Vec<($( $rtype, )*)>, Error> {
                query_internal(connection, None $( , $pname )* $( , $lname )*)
                    .await
                    .map(|(result, _)| result)
                    .context(stringify!(While executing $name query))
            }

            #[allow(dead_code)]
            pub async fn commented_query<'a, C>(
                connection: & Connection,
                comment: C,
                $( $pname: &'a $ptype, )*
                $( $lname: &'a [ $ltype ], )*
            ) -> Result<(Vec<($( $rtype, )*)>, Option<QueryTelemetry>), Error>
            where
                C: Into<Option<&'a str>>,
            {
                let (res, opt_stats) = query_internal(connection, comment.into() $( , $pname )* $( , $lname )*)
                    .await
                    .context(stringify!(While executing $name commented query))?;

                Ok((res, opt_stats))
            }

            #[allow(dead_code)]
            pub async fn query_with_transaction(
                transaction: Transaction,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> Result<(Transaction, Vec<($( $rtype, )*)>), Error> {
                query_internal_with_transaction(transaction, None $( , $pname )* $( , $lname )*)
                    .await
                    .map(|(txn, (result, _))| (txn, result))
                    .context(stringify!(While executing $name query in transaction))
            }

            #[allow(dead_code)]
            pub async fn commented_query_with_transaction<'a, C>(
                transaction: Transaction,
                comment: C,
                $( $pname: &'a $ptype, )*
                $( $lname: &'a [ $ltype ], )*
            ) -> Result<(Transaction, (Vec<($( $rtype, )*)>, Option<QueryTelemetry>)), Error>
            where
                C: Into<Option<&'a str>>,
            {
                query_internal_with_transaction(transaction, comment.into() $( , $pname )* $( , $lname )*)
                    .await
                    .context(stringify!(While executing $name query in transaction))
            }
        }
        $crate::queries!($( $tt )*);
    );

    (
        $vi:vis write $name:ident (
            values: ($( $vname:ident: $vtype:ty ),* $(,)*)
            $( , $pname:ident: $ptype:ty )* $(,)*
        ) { $qtype:ident, $q:expr }
        $( $tt:tt )*
    ) => (
        $crate::queries! {
            $vi write $name (
                values: ($( $vname: $vtype ),*)
                $( , $pname: $ptype )*
            ) { $qtype, mysql($q) sqlite($q) }
            $( $tt )*
        }
    );

    (
        $vi:vis write $name:ident (
            values: ($( $vname:ident: $vtype:ty ),* $(,)*)
            $( , $pname:ident: $ptype:ty )* $(,)*
        ) { $qtype:ident, mysql($mysql_q:expr) sqlite($sqlite_q:expr) }
        $( $tt:tt )*
    ) => (
        #[allow(non_snake_case)]
        $vi mod $name {
            $crate::_write_query_impl!(values: ($( $vname: $vtype ),*), ($( $pname: $ptype ),* ) {
                $qtype,
                mysql($mysql_q)
                sqlite($sqlite_q)
            });

            #[allow(dead_code)]
            pub async fn query(
                connection: &Connection,
                values: &[($( & $vtype, )*)],
                $( $pname: & $ptype ),*
            ) -> Result<WriteResult, Error> {
                query_internal(connection, None, values $( , $pname )*)
                    .await
                    .context(stringify!(While executing $name query))
            }

            #[allow(dead_code)]
            pub async fn commented_query<'a, C>(
                connection: &Connection,
                comment: C,
                values: &'a [($( & $vtype, )*)],
                $( $pname: &'a $ptype ),*
            ) -> Result<WriteResult, Error>
            where
                C: Into<Option<&'a str>>,
            {
                query_internal(connection, comment.into(), values $( , $pname )*)
                    .await
                    .context(stringify!(While executing $name query))
            }

            #[allow(dead_code)]
            pub async fn query_with_transaction(
                transaction: Transaction,
                values: &[($( & $vtype, )*)],
                $( $pname: & $ptype ),*
            ) -> Result<(Transaction, WriteResult), Error> {
                query_internal_with_transaction(transaction, None, values $( , $pname )*)
                    .await
                    .context(stringify!(While executing $name query))
            }

            #[allow(dead_code)]
            pub async fn commented_query_with_transaction<'a, C>(
                transaction: Transaction,
                comment: C,
                values: &'a [($( & $vtype, )*)],
                $( $pname: &'a $ptype ),*
            ) -> Result<(Transaction, WriteResult), Error>
            where
                C: Into<Option<&'a str>>,
            {
                query_internal_with_transaction(transaction, comment.into(), values $( , $pname )*)
                    .await
                    .context(stringify!(While executing $name query))
            }
        }
        $crate::queries!($( $tt )*);
    );

    (
        $vi:vis write $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) { $qtype:ident, $q:expr }
        $( $tt:tt )*
    ) => (
        $crate::queries! {
            $vi write $name (
                $( $pname: $ptype ),*
                $( >list $lname: $ltype )*
            ) { $qtype, mysql($q) sqlite($q) }
            $( $tt )*
        }
    );

    (
        $vi:vis write $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) { $qtype:ident, mysql($mysql_q:expr) sqlite($sqlite_q:expr) }
        $( $tt:tt )*
    ) => (
        #[allow(non_snake_case)]
        $vi mod $name {
            $crate::_write_query_impl!((
                $( $pname: $ptype, )*
                $( >list $lname: $ltype )*
            ) {
                $qtype,
                mysql($mysql_q)
                sqlite($sqlite_q)
            });

            #[allow(dead_code)]
            pub async fn query(
                connection: &Connection,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> Result<WriteResult, Error> {
                query_internal(connection, None $( , $pname )* $( , $lname )*)
                    .await
                    .context(stringify!(While executing $name query))
            }

            #[allow(dead_code)]
            pub async fn commented_query<'a, C>(
                connection: &Connection,
                comment: C,
                $( $pname: &'a $ptype, )*
                $( $lname: &'a [ $ltype ], )*
            ) -> Result<WriteResult, Error>
            where
                C: Into<Option<&'a str>>,
            {
                query_internal(connection, comment.into() $( , $pname )* $( , $lname )*)
                    .await
                    .context(stringify!(While executing $name query))
            }

            #[allow(dead_code)]
            pub async fn query_with_transaction(
                transaction: Transaction,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> Result<(Transaction, WriteResult), Error> {
                query_internal_with_transaction(transaction, None $( , $pname )* $( , $lname )*)
                    .await
                    .context(stringify!(While executing $name query))
            }

            #[allow(dead_code)]
            pub async fn commented_query_with_transaction<'a, C>(
                transaction: Transaction,
                comment: C,
                $( $pname: &'a $ptype, )*
                $( $lname: &'a [ $ltype ], )*
            ) -> Result<(Transaction, WriteResult), Error>
            where
                C: Into<Option<&'a str>>,
            {
                query_internal_with_transaction(transaction, comment.into() $( , $pname )* $( , $lname )*)
                    .await
                    .context(stringify!(While executing $name query))
            }
        }
        $crate::queries!($( $tt )*);
    );
}

#[macro_export]
#[doc(hidden)]
macro_rules! _query_common {
    () => {
        use std::fmt::Write;
        // Some users of queries! have redefined Result
        use std::result::Result;

        use $crate::Connection;
        use $crate::HList;
        use $crate::Transaction;
        use $crate::ValueWrapper;
        use $crate::anyhow::Context;
        use $crate::anyhow::Error;
        use $crate::anyhow::anyhow;
        use $crate::cloned::cloned;
        use $crate::futures::compat::Future01CompatExt;
        use $crate::futures::future::Future;
        use $crate::futures::future::FutureExt;
        use $crate::futures::future::TryFutureExt;
        use $crate::mysql_async::prelude::*;
        use $crate::rusqlite::Connection as SqliteConnection;
        use $crate::rusqlite::Result as SqliteResult;
        use $crate::rusqlite::Row as SqliteRow;
        use $crate::rusqlite::Statement as SqliteStatement;
        use $crate::rusqlite::types::ToSql as ToSqliteValue;
        use $crate::sql_common::QueryTelemetry;
        use $crate::sql_common::mysql::OssConnection;
        use $crate::sqlite::SqliteConnectionGuard;
        use $crate::sqlite::SqliteMultithreaded;
        use $crate::sqlite::SqliteQueryType;

        #[allow(unused_imports)]
        use super::*;
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! _read_query_impl {
    ( (
        $( $pname:ident: $ptype:ty, )*
        $( >list $lname:ident: $ltype:ty )*
    ) -> ($( $rtype:ty ),*) { mysql($mysql_q:expr) sqlite($sqlite_q:expr) } ) => (
        $crate::_query_common!();

        async fn query_internal(
            connection: &Connection,
            comment: Option<&str>,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> Result<(Vec<($( $rtype, )*)>, Option<QueryTelemetry>), Error> {
            match connection {
                Connection::Sqlite(multithread_con) => {
                    let res = sqlite_query(multithread_con $( , $pname )* $( , $lname )*).await?;

                    // Sqlite doesn't support query telemetry
                    Ok((res, None))
                }
                Connection::Mysql(conn) => {
                    let mut query = mysql_query($( $pname, )* $( $lname, )*);
                    if let Some(comment) = comment {
                        query.insert_str(0, &format!("/* {} */", comment));
                    }
                    let (res, tel) = conn.read_query(query).map_err(Error::from).await?;


                    // TODO(T223577767): return telemetry after updating read_query return type
                    #[cfg(fbcode_build)]
                    {

                        Ok((res, tel.map(QueryTelemetry::MySQL)))
                    }

                    #[cfg(not(fbcode_build))]
                    {
                        Ok((res, tel))
                    }
                }
                Connection::OssMysql(conn) => {
                    let query = mysql_query($( $pname, )* $( $lname, )*);

                    let mut con = OssConnection::get_conn_counted(conn.pool.clone(), &conn.stats).await?;
                    let (mut res, _tel) = conn.read_query(&mut con, &query).map_err(Error::from).await?;

                    let result = res
                        .map( |row| mysql_async_row_to_tuple(row))
                        .await?
                        .into_iter()
                        .collect::<Result<Vec<($( $rtype, )*)>, Error>>()?;

                    // OssMysql doesn't support query telemetry
                    Ok((result, None))
                }
            }
        }

        fn mysql_async_row_to_tuple(row: $crate::mysql_async::Row) -> Result<($( $rtype, )*), Error> {
            #[allow(clippy::eval_order_dependence)]
                let mut idx = 0;
                let res = (
                    $({
                        let res: $crate::mysql_async::Value = row.get(idx).ok_or($crate::anyhow::anyhow!("Failed to parse idx"))?;
                        idx += 1;
                        <$rtype as FromValue>::from_value_opt(res)
                            .unwrap_or_else(|err| {
                                panic!("Failed to parse `{}`: {}", stringify!($rtype), err)
                            })
                    },)*
                );
                // suppress unused_assignments warning
                let _ = idx;
                Ok(res)
        }

        async fn query_internal_with_transaction(
            mut transaction: Transaction,
            comment: Option<&str>,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> Result<(Transaction, (Vec<($( $rtype, )*)>, Option<QueryTelemetry>)), Error>{
            match transaction {
                Transaction::Sqlite(ref mut con) => {
                    let con = con
                        .take()
                        .expect("should be Some before transaction ended");

                    sqlite_query_with_transaction(con $( , $pname )* $( , $lname )*)
                        .await
                        .map(move |(con, res)| {
                            // Sqlite doesn't support query telemetry
                            (Transaction::Sqlite(Some(con)), (res, None))
                        })
                }
                Transaction::Mysql(ref mut transaction) => {
                    let mut query = mysql_query($( $pname, )* $( $lname, )*);
                    if let Some(comment) = comment {
                        query.insert_str(0, &format!("/* {} */", comment));
                    }
                    let mut tr = transaction.take()
                        .expect("should be Some before transaction ended");
                    let (result, tel) = tr.read_query(query).map_err(Error::from).await?;

                    // TODO(T223577767): return telemetry after updating read_query return type
                    #[cfg(fbcode_build)]
                    {
                        Ok((Transaction::Mysql(Some(tr)), (result, tel.map(QueryTelemetry::MySQL))))
                    }
                    #[cfg(not(fbcode_build))]
                    {
                        Ok((Transaction::Mysql(Some(tr)), (result, None)))
                    }


                }
                Transaction::OssMysql(ref mut transaction) => {
                    let query = mysql_query($( $pname, )* $( $lname, )*);

                    let mut tr = transaction.take().expect("should be Some before transaction ended");
                    let mut query_result  = tr.query_iter(query).map_err(Error::from).await?;
                    let result = query_result
                        .map(
                        |row| mysql_async_row_to_tuple(row)
                        )
                        .await?
                        .into_iter()
                        .collect::<Result<Vec<($( $rtype, )*)>, Error>>()?;

                    // OssMysql doesn't support query telemetry
                    Ok((Transaction::OssMysql(Some(tr)), (result, None)))
                }
            }
        }

        async fn sqlite_query(
            multithread_con: &SqliteMultithreaded,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> Result<Vec<($( $rtype, )*)>, Error> {
            $crate::_prepare_sqlite_params!(
                params,
                $( $pname ),*
                $( >list $lname )*
            );

            let con = multithread_con.acquire_sqlite_connection(SqliteQueryType::Read).await?;

            let mut ref_params: Vec<(&str, &dyn ToSqliteValue)> = Vec::new();
            for idx in 0..params.len() {
                ref_params.push((&params[idx].0, &params[idx].1))
            }

            sqlite_statement(&con  $( , $lname )*)
                .and_then(|mut stmt| {
                    stmt.query_map(
                        &ref_params[..],
                        sqlite_row_to_tuple
                    )?.collect()
                }).map_err(Error::from)
        }

        async fn sqlite_query_with_transaction(
            transaction: SqliteConnectionGuard,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> Result<(SqliteConnectionGuard, Vec<($( $rtype, )*)>), Error> {
            $crate::_prepare_sqlite_params!(
                params,
                $( $pname ),*
                $( >list $lname )*
            );

            let mut ref_params: Vec<(&str, &dyn ToSqliteValue)> = Vec::new();
            for idx in 0..params.len() {
                ref_params.push((&params[idx].0, &params[idx].1))
            }

            let res: SqliteResult<Vec<($( $rtype, )*)>> = {
                let mut stmt = sqlite_statement(&transaction  $( , $lname )*)?;
                let res = stmt.query_map(
                    &ref_params[..],
                    sqlite_row_to_tuple
                )?.collect();
                res
            };

            Ok((transaction, res?))
        }

        fn mysql_query($( $pname: & $ptype, )* $( $lname: & [ $ltype ], )*) -> String {
            $crate::_emit_mysql_lnames!($( $lname ),*);
            format!(
                $mysql_q,
                $( $pname = ToValue::to_value(&$pname).as_sql(false), )*
                $( $lname = $lname, )*
            )
        }

        fn sqlite_statement<'a>(
            connection: &'a SqliteConnection,
            $( $lname: usize, )*
        ) -> SqliteResult<SqliteStatement<'a>> {
            $crate::_emit_sqlite_lnames!($( $lname ),*);
            connection.prepare(&format!(
                $sqlite_q,
                $( $pname = format!(":{}", stringify!($pname).trim_start_matches("r#")), )*
                $( $lname = $lname, )*
            ))
        }

        fn sqlite_row_to_tuple(row: &SqliteRow) -> SqliteResult<($( $rtype, )*)> {
            // This is currently necessary to use the `mut idx` to keep track of which element of
            // the tuple we are constructing.
            // Once the feature: `macro_metavar_expr` is stable, we can replace `row.get(idx)` with
            // ${index()} and clean up this code a little
            #[allow(clippy::eval_order_dependence)]
            {
                let mut idx = 0;
                let res = (
                    $({
                        let res: ValueWrapper = row.get(idx)?;
                        idx += 1;
                        <$rtype as FromValue>::from_value_opt(res.0)
                            .unwrap_or_else(|err| {
                                panic!("Failed to parse `{}`: {}", stringify!($rtype), err)
                            })
                    },)*
                );
                // suppress unused_assignments warning
                let _ = idx;
                Ok(res)
            }
        }
    );
}

#[macro_export]
#[doc(hidden)]
macro_rules! _write_query_impl {
    ( values: ($( $vname:ident: $vtype:ty ),*), ($( $pname:ident: $ptype:ty ),*) {
        $qtype:ident,
        mysql($mysql_q:expr)
        sqlite($sqlite_q:expr)
    } ) => (
        use $crate::WriteResult;

        $crate::_query_common!();

        async fn query_internal(
            connection: &Connection,
            comment: Option<&str>,
            values: &[($( & $vtype, )*)],
            $( $pname: & $ptype ),*
        ) -> Result<WriteResult, Error> {
            if values.is_empty() {
                return Ok(WriteResult::new(None, 0, None));
            }

            match connection {
                Connection::Sqlite(multithread_con) => {
                    sqlite_exec_query(multithread_con, values, $( $pname ),*).await
                }
                Connection::Mysql(conn) => {
                    let mut query = mysql_query(values, $( $pname ),*);
                    if let Some(comment) = comment {
                        query.insert_str(0, &format!("/* {} */", comment));
                    }
                    let res = conn.write_query(query).map_err(Error::from).await?;
                    Ok(res.into())
                }
                Connection::OssMysql(conn)=> {
                    let query = mysql_query(values, $( $pname ),*);
                    let res = conn.write_query(query).map_err(Error::from).await?;
                    Ok(res.into())
                },
            }
        }

        async fn query_internal_with_transaction(
            mut transaction: Transaction,
            comment: Option<&str>,
            values: &[($( & $vtype, )*)],
            $( $pname: & $ptype ),*
        ) -> Result<(Transaction, WriteResult), Error> {
            if values.is_empty() {
                return Ok((transaction, WriteResult::new(None, 0, None)));
            }

            match transaction {
                Transaction::Sqlite(ref mut transaction) => {
                    let con = transaction
                        .take()
                        .expect("should be Some before transaction ended");

                    sqlite_exec_query_with_transaction(con, values, $( $pname ),*)
                        .await
                        .map(move |(con, res)| {
                            (Transaction::Sqlite(Some(con)), res)
                        })
                }
                Transaction::Mysql(ref mut transaction) => {
                    let mut query = mysql_query(values, $( $pname ),*);
                    if let Some(comment) = comment {
                        query.insert_str(0, &format!("/* {} */", comment));
                    }
                    let mut tr = transaction.take()
                        .expect("should be Some before transaction ended");

                    let result = tr.write_query(query).map_err(Error::from).await?;
                    Ok((Transaction::Mysql(Some(tr)), result.into()))
                },
                Transaction::OssMysql(ref mut transaction)=>{
                    let query = mysql_query(values, $( $pname ),*);
                    let mut tr = transaction.take().expect("should be Some before transaction ended");

                    let query_result = tr.query_iter(query).await?;

                    let last_insert_id = query_result.last_insert_id();
                    let rows_affected = query_result.affected_rows();

                    let result = WriteResult::new(last_insert_id, rows_affected, None);

                    Ok((Transaction::OssMysql(Some(tr)), result.into()))

                },
            }
        }

        fn mysql_query(values: &[($( & $vtype, )*)], $( $pname: & $ptype ),*) -> String {
            let mut val = String::new();
            let mut first = true;
            for value in values {
                if first {
                    first = false;
                } else {
                    write!(&mut val, ", ").unwrap();
                }
                write!(&mut val, "(").unwrap();
                $crate::_append_to_mysql_values!(val, value, $( $vtype, )*);
                write!(&mut val, ")").unwrap();
            }

            $crate::_write_mysql_query!($qtype, $mysql_q, values: val, $( $pname ),*)
        }

        async fn sqlite_exec_query(
            multithread_con: &SqliteMultithreaded,
            values: &[($( & $vtype, )*)],
            $( $pname: & $ptype ),*
        ) -> Result<WriteResult, Error> {
            let mut multi_params = Vec::new();
            for value in values {
                let mut params: Vec<(String, ValueWrapper)> = Vec::new();

                $crate::_sqlite_named_params!(params, value $( , $vname )*);
                $(
                    params.push((
                        format!(":{}", stringify!($pname).trim_start_matches("r#")),
                        ValueWrapper(ToValue::to_value($pname)),
                    ));
                )*

                multi_params.push(params);
            }

            let con = multithread_con.acquire_sqlite_connection(SqliteQueryType::Write).await?;

            let mut stmt = sqlite_statement(&con)?;

            let mut res = Vec::new();
            for params in multi_params {
                let mut param_refs: Vec<(&str, &dyn ToSqliteValue)> = Vec::new();
                for param in &params {
                    param_refs.push((param.0.as_str(), &param.1));
                }

                let a: &[(&str, &dyn ToSqliteValue)] = &param_refs[..];
                res.push(stmt.execute(a)?);
            }

            Ok(WriteResult::new(
                Some(con.last_insert_rowid() as u64),
                res.into_iter().sum::<usize>() as u64,
                None,
            ))
        }

        async fn sqlite_exec_query_with_transaction(
            transaction: SqliteConnectionGuard,
            values: &[($( & $vtype, )*)],
            $( $pname: & $ptype ),*
        ) -> Result<(SqliteConnectionGuard, WriteResult), Error> {
            let mut multi_params = Vec::new();
            for value in values {
                let mut params: Vec<(String, ValueWrapper)> = Vec::new();

                $crate::_sqlite_named_params!(params, value $( , $vname )*);
                $(
                    params.push((
                        format!(":{}", stringify!($pname).trim_start_matches("r#")),
                        ValueWrapper(ToValue::to_value($pname)),
                    ));
                )*

                multi_params.push(params);
            }

            let res: usize = {
                let mut stmt = sqlite_statement(&transaction)?;

                let mut res = Vec::new();
                for params in multi_params {
                    let mut param_refs: Vec<(&str, &dyn ToSqliteValue)> = Vec::new();
                    for param in &params {
                        param_refs.push((param.0.as_str(), &param.1));
                    }

                    let a: &[(&str, &dyn ToSqliteValue)] = &param_refs[..];
                    res.push(stmt.execute(a)?);
                }

                res.into_iter().sum::<usize>()
            };

            let res = WriteResult::new(
                Some(transaction.last_insert_rowid() as u64),
                res as u64,
                None,
            );

            Ok((transaction, res))
        }

        fn sqlite_statement<'a>(
            connection: &'a SqliteConnection,
        ) -> SqliteResult<SqliteStatement<'a>> {
            let mut val = Vec::new();
            $(
                val.push(format!(":{}", stringify!($vname).trim_start_matches("r#")));
            )*
            connection.prepare(&$crate::_write_sqlite_query!(
                $qtype,
                $sqlite_q,
                values: &format!("({})", val.join(", ")),
                $( $pname ),*
            ))
        }
    );

    ( (
        $( $pname:ident: $ptype:ty, )*
        $( >list $lname:ident: $ltype:ty )*
    ) { $qtype:ident, mysql($mysql_q:expr) sqlite($sqlite_q:expr) } ) => (
        use $crate::WriteResult;

        $crate::_query_common!();

        async fn query_internal(
            connection: &Connection,
            comment: Option<&str>,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> Result<WriteResult, Error> {
            match connection {
                Connection::Sqlite(multithread_con) => {
                    sqlite_exec_query(multithread_con $( , $pname )* $( , $lname )*).await
                }
                Connection::Mysql(conn) => {
                    let mut query = mysql_query($( $pname, )* $( $lname, )*);
                    if let Some(comment) = comment {
                        query.insert_str(0, &format!("/* {} */", comment));
                    }
                    let res = conn.write_query(query).map_err(Error::from).await?;
                    Ok(res.into())
                }
                Connection::OssMysql(conn) => {
                    let query = mysql_query($( $pname, )* $( $lname, )*);
                    let res = conn.write_query(query).map_err(Error::from).await?;
                    Ok(res.into())
                },
            }
        }

        async fn query_internal_with_transaction(
            mut transaction: Transaction,
            comment: Option<&str>,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> Result<(Transaction, WriteResult), Error> {
            match transaction {
                Transaction::Sqlite(ref mut transaction) => {
                    let con = transaction
                        .take()
                        .expect("should be Some before transaction ended");

                    sqlite_exec_query_with_transaction(con $( , $pname )* $( , $lname )*)
                        .await
                        .map(move |(con, res)| {
                            (Transaction::Sqlite(Some(con)), res)
                        })
                }
                Transaction::Mysql(ref mut transaction) => {
                    let mut query = mysql_query($( $pname, )* $( $lname, )*);
                    if let Some(comment) = comment {
                        query.insert_str(0, &format!("/* {} */", comment));
                    }
                    let mut tr = transaction.take()
                        .expect("should be Some before transaction ended");
                    let result = tr.write_query(query).map_err(Error::from).await?;
                    Ok((Transaction::Mysql(Some(tr)), result.into()))
                },
                Transaction::OssMysql(ref mut transaction) => {
                    let query = mysql_query($( $pname, )* $( $lname, )*);
                    let mut tr = transaction.take()
                        .expect("should be Some before transaction ended");
                    let query_result = tr.query_iter(query).await?;

                    let last_insert_id = query_result.last_insert_id();
                    let rows_affected = query_result.affected_rows();
                    let result = WriteResult::new(last_insert_id, rows_affected, None);
                    Ok((Transaction::OssMysql(Some(tr)), result))
                }
            }
        }

        fn mysql_query($( $pname: & $ptype, )* $( $lname: & [ $ltype ], )*) -> String {
            $crate::_emit_mysql_lnames!($( $lname ),*);
            $crate::_write_mysql_query!($qtype, $mysql_q, $( $pname ),* $( >list $lname )*)
        }

        async fn sqlite_exec_query(
            multithread_con: &SqliteMultithreaded,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> Result<WriteResult, Error> {
            $crate::_prepare_sqlite_params!(
                params,
                $( $pname ),*
                $( >list $lname )*
            );

            let con = multithread_con.acquire_sqlite_connection(SqliteQueryType::Write).await?;

            let mut stmt = sqlite_statement(&con  $( , $lname )*)?;

            let mut param_refs: Vec<(&str, &dyn ToSqliteValue)> = Vec::new();
            for param in &params {
                param_refs.push((&param.0, &param.1));
            }

                    let a: &[(&str, &dyn ToSqliteValue)] = &param_refs[..];
                    let res = stmt.execute(a)?;

            Ok(WriteResult::new(
                Some(con.last_insert_rowid() as u64),
                res as u64,
                None,
            ))
        }

        async fn sqlite_exec_query_with_transaction(
            transaction: SqliteConnectionGuard,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> Result<(SqliteConnectionGuard, WriteResult), Error> {
            $crate::_prepare_sqlite_params!(
                params,
                $( $pname ),*
                $( >list $lname )*
            );

            let res = {
                let mut stmt = sqlite_statement(&transaction  $( , $lname )*)?;

                let mut param_refs: Vec<(&str, &dyn ToSqliteValue)> = Vec::new();
                for param in &params {
                    param_refs.push((&param.0, &param.1));
                }

                    let a: &[(&str, &dyn ToSqliteValue)] = &param_refs[..];
                    stmt.execute(a)?
            };

            let res = WriteResult::new(
                Some(transaction.last_insert_rowid() as u64),
                res as u64,
                None,
            );

            Ok((transaction, res))
        }

        fn sqlite_statement<'a>(
            connection: &'a SqliteConnection,
            $( $lname: usize, )*
        ) -> SqliteResult<SqliteStatement<'a>> {
            $crate::_emit_sqlite_lnames!($( $lname ),*);
            connection.prepare(&$crate::_write_sqlite_query!(
                $qtype,
                $sqlite_q,
                $( $pname ),*
                $( >list $lname )*
            ))
        }
    );
}

#[macro_export]
#[doc(hidden)]
macro_rules! _write_mysql_query {
    (insert_or_ignore, $q:expr, values: $values:expr, $( $pname:ident ),*) => {
        format!(
            $q,
            insert_or_ignore = "INSERT IGNORE",
            values = $values,
            $( $pname = ToValue::to_value(&$pname).as_sql(false), )*
        )
    };

    (insert_or_ignore, $q:expr, $( $pname:ident ),* $( >list $lname:ident )*) => {
        format!(
            $q,
            insert_or_ignore = "INSERT IGNORE",
            $( $pname = ToValue::to_value(&$pname).as_sql(false), )*
            $( $lname = $lname, )*
        )
    };

    (none, $q:expr, values: $values:expr, $( $pname:ident ),*) => {
        format!(
            $q,
            values = $values,
            $( $pname = ToValue::to_value(&$pname).as_sql(false), )*
        )
    };

    (none, $q:expr, $( $pname:ident ),* $( >list $lname:ident )*) => {
        format!(
            $q,
            $( $pname = ToValue::to_value(&$pname).as_sql(false), )*
            $( $lname = $lname, )*
        )
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! _write_sqlite_query {
    (insert_or_ignore, $q:expr, values: $values:expr, $( $pname:ident ),*) => {
        format!(
            $q,
            insert_or_ignore = "INSERT OR IGNORE",
            values = $values,
            $( $pname = format!(":{}", stringify!($pname).trim_start_matches("r#")), )*
        )
    };

    (insert_or_ignore, $q:expr, $( $pname:ident ),* $( >list $lname:ident )*) => {
        format!(
            $q,
            insert_or_ignore = "INSERT OR IGNORE",
            $( $pname = format!(":{}", stringify!($pname).trim_start_matches("r#")), )*
            $( $lname = $lname, )*
        )
    };

    (none, $q:expr, values: $values:expr, $( $pname:ident ),*) => {
        format!(
            $q,
            values = $values,
            $( $pname = format!(":{}", stringify!($pname).trim_start_matches("r#")), )*
        )
    };

    (none, $q:expr, $( $pname:ident ),* $( >list $lname:ident )*) => {
        format!(
            $q,
            $( $pname = format!(":{}", stringify!($pname).trim_start_matches("r#")), )*
            $( $lname = $lname, )*
        )
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! _sqlite_named_params {
    ($params:ident, $tup:ident $( , $vname:ident )*) => (
        $crate::_sqlite_named_params!(@expand () {} $params, $tup $( , $vname )*);
    );

    (
        @expand
        ( $( $binds:pat , )* )
        { $( $unames:ident => $uses:expr , )* }
        $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*
    ) => (
        $crate::_sqlite_named_params!(
            @expand
            ( $( $binds , )* value , )
            { $( $unames => $uses , )* $vname => value , }
            $params, $tup $( , $vnames )*
        )
    );

    (
        @expand
        ( $( $binds:pat , )* )
        { $( $unames:ident => $uses:expr , )* }
        $params:ident, $tup:ident
    ) => (
        match $tup {
            ( $( $binds , )* ) => {
                $(
                    $params.push((
                        format!(":{}", stringify!($unames).trim_start_matches("r#")),
                        ValueWrapper(ToValue::to_value($uses)),
                    ));
                )*
            }
        }
    );
}

#[macro_export]
#[doc(hidden)]
macro_rules! _append_to_mysql_values {
    ($values:ident, $tup:ident, $( $vtype:ty, )*) => (
        $crate::_append_to_mysql_values!(@expand () {} $values, $tup, $( $vtype, )* )
    );

    (
        @expand
        ( $( $binds:pat , )* )
        { $( $uses:expr , )* }
        $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+
    ) => (
        $crate::_append_to_mysql_values!(
            @expand
            ( $( $binds , )* value , )
            { $( $uses , )* value , }
            $values, $tup, $( $vtypes, )+
        )
    );

    (
        @expand
        ( $( $binds:pat , )* )
        { $( $uses:expr , )* }
        $values:ident, $tup:ident, $vtype:ty,
    ) => (
        match $tup {
            ( $( $binds , )* value , ) => {
                $(
                    write!(&mut $values, "{}, ", $uses.to_value().as_sql(false)).unwrap();
                )*
                write!(&mut $values, "{}", value.to_value().as_sql(false)).unwrap();
            }
        }
    );
}

#[macro_export]
#[doc(hidden)]
/// Serialize all >list $lname elements into strings suitable for interpolation into a SQL string.
macro_rules! _emit_mysql_lnames {
    ($( $lname:ident ),*) => {
        $(
            let $lname = {
                let mut val = String::new();
                write!(&mut val, "(").unwrap();
                let mut first = true;
                for lval in $lname {
                    if first {
                        first = false;
                    } else {
                        write!(&mut val, ", ").unwrap();
                    }
                    write!(&mut val, "{}", ToValue::to_value(&lval).as_sql(false)).unwrap();
                }
                write!(&mut val, ")").unwrap();
                val
            };
        )*
    }
}

#[macro_export]
#[doc(hidden)]
/// Serialize all >list $lname elements into strings suitable for interpolation into a SQLite
/// prepared statement.
macro_rules! _emit_sqlite_lnames {
    ($( $lname:ident ),*) => {
        $(
            let $lname = {
                let mut val = String::new();
                write!(&mut val, "(").unwrap();
                let mut first = true;
                for idx in 0..$lname {
                    if first {
                        first = false;
                    } else {
                        write!(&mut val, ", ").unwrap();
                    }
                    write!(&mut val, concat!(":", stringify!($lname), "{}"), idx).unwrap();
                }
                write!(&mut val, ")").unwrap();
                val
            };
        )*
    }
}

#[macro_export]
#[doc(hidden)]
/// Prepares $params for a SQLite query.
macro_rules! _prepare_sqlite_params {
    ($params:ident, $( $pname:ident ),* $( >list $lname:ident )*) => (
        let $params = vec![ $(
            (format!(":{}", stringify!($pname).trim_start_matches("r#")), ValueWrapper(ToValue::to_value($pname)))
        ),* ].into_iter();

        $(
            let $params = $params.chain(
                $lname.into_iter()
                    .enumerate()
                    .map(|(idx, val)| (
                        format!(":{}{}", stringify!($lname), idx),
                        ValueWrapper(ToValue::to_value(val)),
                    ))
            );
        )*

        let $params: Vec<(String, ValueWrapper)> = $params.collect();

        $(
            let $lname = $lname.len();
        )*
    )
}

#[macro_export]
/// Given types `T`, `Intermediate`, and `Raw` as macro arguments:
///   * Define `FromValue` for `T` with `Intermedate` set as the associated type.
///   * Define `From<T>` for `Value` in terms of `impl From<T> for Raw`
///   * Derive an impl for `ConvIr<T>` for `Intermediate` by delegating to the
///     `ConvIr<Raw>` impl
///
/// Requires the following constraints:
///    * `Intermediate` already implements `ConvIr<Raw>`
///    * `T` implements both `From<Raw>` and `Into<Raw>`
///
/// # Example:
/// ```ignore
/// use derive_more::{From, Into};
/// use mysql_common::value::convert::ParseIr;
///
/// #[derive(From, Into, mysql::OptTryFromRowField)]
/// pub struct MyID(u64);
/// sql::proxy_conv_ir!(MyID, ParseIr<u64>, u64);
/// ```
/// will expand to the equivalent of:
/// ```ignore
/// use derive_more::{From, Into};
/// use mysql_common::value::convert::ParseIr;
///
/// #[derive(From, Into, mysql::OptTryFromRowField)]
/// pub struct MyID(u64);
/// impl mysql_async::prelude::FromValue for MyId {
///    type Intermediate = ParseIr<u64>
/// }
/// impl From<MyId> for mysql_async::Value {
///     fn from(f: MyId) -> mysql_async::Value {
///         let r: u64 = f.into();
///         r.into()
///     }
/// }
/// impl mysql_async::prelude::ConvIr<MyId> for ParseIr<u64> {
///   // delegates `new`, `commit` and `rollback` from ConvIr<MyId> to ConvIr<u64>
///   // Includes type conversions between u64 and MyId via `into` where needed
/// }
/// ```
macro_rules! proxy_conv_ir {
    ($t:ty, $intermediate:ty, $raw:ty) => {
        impl $crate::mysql_async::prelude::FromValue for $t {
            type Intermediate = $intermediate;
        }

        impl From<$t> for $crate::mysql_async::Value {
            #[inline(always)]
            fn from(f: $t) -> $crate::mysql_async::Value {
                let r: $raw = f.into();
                r.into()
            }
        }

        impl $crate::mysql_async::prelude::ConvIr<$t> for $intermediate {
            #[inline(always)]
            fn new(
                v: $crate::mysql_async::Value,
            ) -> Result<$intermediate, $crate::mysql_async::FromValueError> {
                $crate::mysql_async::prelude::ConvIr::<$raw>::new(v)
            }

            #[inline(always)]
            fn commit(self) -> $t {
                $crate::mysql_async::prelude::ConvIr::<$raw>::commit(self).into()
            }

            #[inline(always)]
            fn rollback(self) -> $crate::mysql_async::Value {
                $crate::mysql_async::prelude::ConvIr::<$raw>::rollback(self)
            }
        }
    };
}
