/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
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
//! use futures::Future;
//!
//! use sql::{queries, Connection};
//! use sql_tests_lib::{A, B};
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
//! fn foo(conn: Connection) {
//!     assert_eq!(
//!         MySelect::query(&conn, &A, &72).wait().unwrap(),
//!         vec![(44, B, B, 72)]
//!     );
//!
//!     let res = MyInsert::query(&conn, &[(&44,)]).wait().unwrap();
//!     assert_eq!(res.affected_rows(), 1);
//!     assert_eq!(res.last_insert_id(), Some(1));
//! }
//! #
//! # fn main() {}
//! ```

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

#[cfg(test)]
mod tests;

pub use anyhow;
pub use failure;
pub use failure_ext;
pub use futures;
pub use futures_ext;
pub use mysql_async;
pub use rusqlite;
pub use sql_common::{self, error, sqlite, transaction::Transaction, Connection, WriteResult};

use mysql_async::Value;
use rusqlite::types::{
    FromSql as FromSqliteValue, FromSqlResult as FromSqliteValueResult, ToSql as ToSqliteValue,
    ToSqlOutput as ToSqliteOutput, Value as SqliteValue, ValueRef as SqliteValueRef,
};
use rusqlite::Result as SqliteResult;

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
            Value::Float(f) => ToSqliteOutput::Owned(SqliteValue::Real(*f)),
            Value::Date(..) | Value::Time(..) => {
                unimplemented!("TODO(luk) implement date and time for sqlite")
            }
        })
    }
}

impl FromSqliteValue for ValueWrapper {
    fn column_result(value: SqliteValueRef<'_>) -> FromSqliteValueResult<Self> {
        Ok(ValueWrapper(match value {
            SqliteValueRef::Null => Value::NULL,
            SqliteValueRef::Integer(i) => Value::Int(i),
            SqliteValueRef::Real(f) => Value::Float(f),
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
        read $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) -> ($( $rtype:ty ),* $(,)*) { $q:expr }
        $( $tt:tt )*
    ) => (
        $crate::queries! {
            read $name (
                $( $pname: $ptype ),*
                $( >list $lname: $ltype )*
            ) -> ($( $rtype ),*) { mysql($q) sqlite($q) }
            $( $tt )*
        }
    );

    (
        read $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) -> ($( $rtype:ty ),* $(,)*) { mysql($mysql_q:expr) sqlite($sqlite_q:expr) }
        $( $tt:tt )*
    ) => (
        #[allow(non_snake_case)]
        mod $name {
            $crate::_read_query_impl!((
                $( $pname: $ptype, )*
                $( >list $lname: $ltype )*
            ) -> ($( $rtype ),*) { mysql($mysql_q) sqlite($sqlite_q) });

            #[allow(dead_code)]
            pub(super) fn query(
                connection: &Connection,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> impl Future<Item = Vec<($( $rtype, )*)>, Error = Error> {
                query_internal(connection $( , $pname )* $( , $lname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
            }

            #[allow(dead_code)]
            pub(super) fn query_with_transaction(
                transaction: Transaction,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> impl Future<Item = (Transaction, Vec<($( $rtype, )*)>), Error = Error> {
                query_internal_with_transaction(transaction $( , $pname )* $( , $lname )*)
                    .chain_err(stringify!(While executing $name query in transaction))
                    .from_err()
            }
        }
        $crate::queries!($( $tt )*);
    );

    (
        pub read $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) -> ($( $rtype:ty ),* $(,)*) { $q:expr }
        $( $tt:tt )*
    ) => (
        $crate::queries! {
            pub read $name (
                $( $pname: $ptype ),*
                $( >list $lname: $ltype )*
            ) -> ($( $rtype ),*) { mysql($q) sqlite($q) }
            $( $tt )*
        }
    );

    (
        pub read $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) -> ($( $rtype:ty ),* $(,)*) { mysql($mysql_q:expr) sqlite($sqlite_q:expr) }
        $( $tt:tt )*
    ) => (
        #[allow(non_snake_case)]
        pub mod $name {
            $crate::_read_query_impl!((
                $( $pname: $ptype, )*
                $( >list $lname: $ltype )*
            ) -> ($( $rtype ),*) { mysql($mysql_q) sqlite($sqlite_q) });

            #[allow(dead_code)]
            pub fn query(
                connection: &Connection,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> impl Future<Item = Vec<($( $rtype, )*)>, Error = Error> {
                query_internal(connection $( , $pname )* $( , $lname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
            }

            #[allow(dead_code)]
            pub fn query_with_transaction(
                transaction: Transaction,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> impl Future<Item = (Transaction, Vec<($( $rtype, )*)>), Error = Error> {
                query_internal_with_transaction(transaction $( , $pname )* $( , $lname )*)
                    .chain_err(stringify!(While executing $name query in transaction))
                    .from_err()
            }
        }
        $crate::queries!($( $tt )*);
    );

    (
        write $name:ident (
            values: ($( $vname:ident: $vtype:ty ),* $(,)*)
            $( , $pname:ident: $ptype:ty )* $(,)*
        ) { $qtype:ident, $q:expr }
        $( $tt:tt )*
    ) => (
        $crate::queries! {
            write $name (
                values: ($( $vname: $vtype ),*)
                $( , $pname: $ptype )*
            ) { $qtype, mysql($q) sqlite($q) }
            $( $tt )*
        }
    );

    (
        write $name:ident (
            values: ($( $vname:ident: $vtype:ty ),* $(,)*)
            $( , $pname:ident: $ptype:ty )* $(,)*
        ) { $qtype:ident, mysql($mysql_q:expr) sqlite($sqlite_q:expr) }
        $( $tt:tt )*
    ) => (
        #[allow(non_snake_case)]
        mod $name {
            $crate::_write_query_impl!(values: ($( $vname: $vtype ),*), ($( $pname: $ptype ),* ) {
                $qtype,
                mysql($mysql_q)
                sqlite($sqlite_q)
            });

            #[allow(dead_code)]
            pub(super) fn query(
                connection: &Connection,
                values: &[($( & $vtype, )*)],
                $( $pname: & $ptype ),*
            ) -> impl Future<Item = WriteResult, Error = Error> {
                query_internal(connection, values $( , $pname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
            }

            #[allow(dead_code)]
            pub(super) fn query_with_transaction(
                transaction: Transaction,
                values: &[($( & $vtype, )*)],
                $( $pname: & $ptype ),*
            ) -> impl Future<Item = (Transaction, WriteResult), Error = Error> {
                query_internal_with_transaction(transaction, values $( , $pname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
            }
        }
        $crate::queries!($( $tt )*);
    );

    (
        pub write $name:ident (
            values: ($( $vname:ident: $vtype:ty ),* $(,)*)
            $( , $pname:ident: $ptype:ty )* $(,)*
        ) { $qtype:ident, $q:expr }
        $( $tt:tt )*
    ) => (
        $crate::queries! {
            pub write $name (
                values: ($( $vname: $vtype ),*)
                $( , $pname: $ptype )*
            ) { $qtype, mysql($q) sqlite($q) }
            $( $tt )*
        }
    );

    (
        pub write $name:ident (
            values: ($( $vname:ident: $vtype:ty ),* $(,)*)
            $( , $pname:ident: $ptype:ty )* $(,)*
        ) { $qtype:ident, mysql($mysql_q:expr) sqlite($sqlite_q:expr) }
        $( $tt:tt )*
    ) => (
        #[allow(non_snake_case)]
        pub mod $name {
            $crate::_write_query_impl!(values: ($( $vname: $vtype ),*), ($( $pname: $ptype ),* ) {
                $qtype,
                mysql($mysql_q)
                sqlite($sqlite_q)
            });

            #[allow(dead_code)]
            pub fn query(
                connection: &Connection,
                values: &[($( & $vtype, )*)],
                $( $pname: & $ptype ),*
            ) -> impl Future<Item = WriteResult, Error = Error> {
                query_internal(connection, values $( , $pname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
            }

            #[allow(dead_code)]
            pub fn query_with_transaction(
                transaction: Transaction,
                values: &[($( & $vtype, )*)],
                $( $pname: & $ptype ),*
            ) -> impl Future<Item = (Transaction, WriteResult), Error = Error> {
                query_internal_with_transaction(transaction, values $( , $pname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
            }
        }
        $crate::queries!($( $tt )*);
    );

    (
        write $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) { $qtype:ident, $q:expr }
        $( $tt:tt )*
    ) => (
        $crate::queries! {
            write $name (
                $( $pname: $ptype ),*
                $( >list $lname: $ltype )*
            ) { $qtype, mysql($q) sqlite($q) }
            $( $tt )*
        }
    );

    (
        write $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) { $qtype:ident, mysql($mysql_q:expr) sqlite($sqlite_q:expr) }
        $( $tt:tt )*
    ) => (
        #[allow(non_snake_case)]
        mod $name {
            $crate::_write_query_impl!((
                $( $pname: $ptype, )*
                $( >list $lname: $ltype )*
            ) {
                $qtype,
                mysql($mysql_q)
                sqlite($sqlite_q)
            });

            #[allow(dead_code)]
            pub(super) fn query(
                connection: &Connection,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> impl Future<Item = WriteResult, Error = Error> {
                query_internal(connection $( , $pname )* $( , $lname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
            }

            #[allow(dead_code)]
            pub(super) fn query_with_transaction(
                transaction: Transaction,
                $( $pname: & $ptype, )*
                $( $lname: & [ $ltype ], )*
            ) -> impl Future<Item = (Transaction, WriteResult), Error = Error> {
                query_internal_with_transaction(transaction $( , $pname )* $( , $lname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
            }
        }
        $crate::queries!($( $tt )*);
    );

    (
        pub write $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) { $qtype:ident, $q:expr }
        $( $tt:tt )*
    ) => (
        $crate::queries! {
            pub write $name (
                $( $pname: $ptype ),*
                $( >list $lname: $ltype )*
            ) { $qtype, mysql($q) sqlite($q) }
            $( $tt )*
        }
    );

    (
        pub write $name:ident (
            $( $pname:ident: $ptype:ty ),* $(,)*
            $( >list $lname:ident: $ltype:ty )*
        ) { $qtype:ident, mysql($mysql_q:expr) sqlite($sqlite_q:expr) }
        $( $tt:tt )*
    ) => (
        #[allow(non_snake_case)]
        pub mod $name {
            $crate::_write_query_impl!(($( $pname: $ptype ),* ) {
                $qtype,
                mysql($mysql_q)
                sqlite($sqlite_q)
            });

            #[allow(dead_code)]
            pub fn query(
                connection: &Connection,
                $( $pname: & $ptype ),*
            ) -> impl Future<Item = WriteResult, Error = Error> {
                query_internal(connection, $( , $pname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
            }

            #[allow(dead_code)]
            pub fn query_with_transaction(
                transaction: Transaction,
                $( $pname: & $ptype ),*
            ) -> impl Future<Item = (Transaction, WriteResult), Error = Error> {
                query_internal_with_transaction(transaction $( , $pname )*)
                    .chain_err(stringify!(While executing $name query))
                    .from_err()
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
        use std::sync::Arc;

        use $crate::anyhow::Error;
        use $crate::failure::Fail;
        use $crate::failure_ext::chain::ChainExt;
        use $crate::futures::{
            future::{lazy, IntoFuture},
            Future,
        };
        use $crate::futures_ext::{BoxFuture, FutureExt};
        use $crate::mysql_async::prelude::*;
        use $crate::rusqlite::{
            types::ToSql as ToSqliteValue, Connection as SqliteConnection, Result as SqliteResult,
            Statement as SqliteStatement,
        };
        use $crate::sql_common::{
            error::from_failure,
            mysql::{MysqlConnectionExt, MysqlTransactionExt},
        };
        use $crate::{
            sqlite::{SqliteConnectionGuard, SqliteMultithreaded},
            Connection, Transaction, ValueWrapper,
        };

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

        fn query_internal(
            connection: &Connection,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> BoxFuture<Vec<($( $rtype, )*)>, Error> {
            match connection {
                Connection::Sqlite(multithread_con) => {
                    sqlite_query(multithread_con.clone() $( , $pname )* $( , $lname )*)
                }
                Connection::Mysql(con) => {
                    con.read_query(mysql_query($( $pname, )* $( $lname, )*))
                }
            }
        }

        fn query_internal_with_transaction(
            mut transaction: Transaction,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> BoxFuture<(Transaction, Vec<($( $rtype, )*)>), Error> {
            match transaction {
                Transaction::Sqlite(ref mut con) => {
                    let con = con
                        .take()
                        .expect("should be Some before transaction ended");

                    sqlite_query_with_transaction(con $( , $pname )* $( , $lname )*)
                        .map(move |(con, res)| {
                            (Transaction::Sqlite(Some(con)), res)
                        })
                        .boxify()
                }
                Transaction::Mysql(ref mut transaction) => {
                    let transaction = transaction.take()
                        .expect("should be Some before transaction ended");
                    transaction
                        .read_query(mysql_query($( $pname, )* $( $lname, )*))
                        .map(|(tra, result)| (Transaction::Mysql(Some(tra)), result))
                        .boxify()
                }
            }
        }

        fn sqlite_query(
            multithread_con: Arc<SqliteMultithreaded>,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> BoxFuture<Vec<($( $rtype, )*)>, Error> {
            $crate::_prepare_sqlite_params!(
                params,
                $( $pname ),*
                $( >list $lname )*
            );

            lazy(move || {
                let con = multithread_con.get_sqlite_guard();

                let mut ref_params: Vec<(&str, &ToSqliteValue)> = Vec::new();
                for idx in 0..params.len() {
                    ref_params.push((&params[idx].0, &params[idx].1))
                }

                let res = sqlite_statement(&con  $( , $lname )*)
                    .and_then(|mut stmt| {
                        stmt.query_map_named(
                            &ref_params[..],
                            |row| {
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
                        })?.collect()
                    });
                res
            })
                .from_err()
                .boxify()
        }

        fn sqlite_query_with_transaction(
            transaction: SqliteConnectionGuard,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> BoxFuture<(SqliteConnectionGuard, Vec<($( $rtype, )*)>), Error> {
            $crate::_prepare_sqlite_params!(
                params,
                $( $pname ),*
                $( >list $lname )*
            );

            lazy(move || -> SqliteResult<(SqliteConnectionGuard, Vec<($( $rtype, )*)>)> {
                let mut ref_params: Vec<(&str, &ToSqliteValue)> = Vec::new();
                for idx in 0..params.len() {
                    ref_params.push((&params[idx].0, &params[idx].1))
                }

                let res: SqliteResult<Vec<($( $rtype, )*)>> = {
                    let mut stmt = sqlite_statement(&transaction  $( , $lname )*)?;
                    let res = stmt.query_map_named(
                        &ref_params[..],
                        |row| {
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
                    })?.collect();
                    res
                };

                Ok((transaction, res?))
            })
                .from_err()
                .boxify()
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
                $( $pname = concat!(":", stringify!($pname)), )*
                $( $lname = $lname, )*
            ))
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

        fn query_internal(
            connection: &Connection,
            values: &[($( & $vtype, )*)],
            $( $pname: & $ptype ),*
        ) -> BoxFuture<WriteResult, Error> {
            if values.is_empty() {
                return Ok(WriteResult::new(None, 0)).into_future().boxify();
            }

            match connection {
                Connection::Sqlite(multithread_con) => {
                    sqlite_exec_query(multithread_con.clone(), values, $( $pname ),*)
                }
                Connection::Mysql(con) => {
                    con.write_query(mysql_query(values, $( $pname ),*))
                }
            }
        }

        fn query_internal_with_transaction(
            mut transaction: Transaction,
            values: &[($( & $vtype, )*)],
            $( $pname: & $ptype ),*
        ) -> BoxFuture<(Transaction, WriteResult), Error> {
            if values.is_empty() {
                return Ok((transaction, WriteResult::new(None, 0))).into_future().boxify();
            }

            match transaction {
                Transaction::Sqlite(ref mut transaction) => {
                    let con = transaction
                        .take()
                        .expect("should be Some before transaction ended");

                    sqlite_exec_query_with_transaction(con, values, $( $pname ),*)
                        .map(move |(con, res)| {
                            (Transaction::Sqlite(Some(con)), res)
                        })
                        .boxify()
                }
                Transaction::Mysql(ref mut transaction) => {
                    let transaction = transaction
                        .take()
                        .expect("should be Some before transaction ended");
                    transaction
                        .write_query(mysql_query(values, $( $pname ),*))
                        .map(|(tra, result)| (Transaction::Mysql(Some(tra)), result))
                        .boxify()
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

        fn sqlite_exec_query(
            multithread_con: Arc<SqliteMultithreaded>,
            values: &[($( & $vtype, )*)],
            $( $pname: & $ptype ),*
        ) -> BoxFuture<WriteResult, Error> {
            let mut multi_params = Vec::new();
            for value in values {
                let mut params: Vec<(&str, ValueWrapper)> = Vec::new();

                $crate::_sqlite_named_params!(params, value $( , $vname )*);
                $(
                    params.push((
                        concat!(":", stringify!($pname)),
                        ValueWrapper(ToValue::to_value($pname)),
                    ));
                )*

                multi_params.push(params);
            }

            lazy(move || -> SqliteResult<WriteResult> {
                let con = multithread_con.get_sqlite_guard();

                let mut stmt = sqlite_statement(&con)?;

                let mut res = Vec::new();
                for params in multi_params {
                    let mut param_refs: Vec<(&str, &ToSqliteValue)> = Vec::new();
                    for param in &params {
                        param_refs.push((param.0, &param.1));
                    }

                    res.push(stmt.execute_named(param_refs.as_ref())?);
                }

                Ok(WriteResult::new(
                    Some(con.last_insert_rowid() as u64),
                    res.into_iter().sum::<usize>() as u64,
                ))
            })
                .from_err()
                .boxify()
        }

        fn sqlite_exec_query_with_transaction(
            transaction: SqliteConnectionGuard,
            values: &[($( & $vtype, )*)],
            $( $pname: & $ptype ),*
        ) -> BoxFuture<(SqliteConnectionGuard, WriteResult), Error> {
            let mut multi_params = Vec::new();
            for value in values {
                let mut params: Vec<(&str, ValueWrapper)> = Vec::new();

                $crate::_sqlite_named_params!(params, value $( , $vname )*);
                $(
                    params.push((
                        concat!(":", stringify!($pname)),
                        ValueWrapper(ToValue::to_value($pname)),
                    ));
                )*

                multi_params.push(params);
            }

            lazy(move || -> SqliteResult<(SqliteConnectionGuard, WriteResult)> {
                let res = {
                    let mut stmt = sqlite_statement(&transaction)?;

                    let mut res = Vec::new();
                    for params in multi_params {
                        let mut param_refs: Vec<(&str, &ToSqliteValue)> = Vec::new();
                        for param in &params {
                            param_refs.push((param.0, &param.1));
                        }

                        res.push(stmt.execute_named(param_refs.as_ref())?);
                    }

                    res.into_iter().sum::<usize>()
                };

                let res = WriteResult::new(
                    Some(transaction.last_insert_rowid() as u64),
                    res as u64,
                );

                Ok((transaction, res))
            })
                .from_err()
                .boxify()
        }

        fn sqlite_statement<'a>(
            connection: &'a SqliteConnection,
        ) -> SqliteResult<SqliteStatement<'a>> {
            let mut val = Vec::new();
            $(
                val.push(concat!(":", stringify!($vname)));
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

        fn query_internal(
            connection: &Connection,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> BoxFuture<WriteResult, Error> {
            match connection {
                Connection::Sqlite(multithread_con) => {
                    sqlite_exec_query(multithread_con.clone() $( , $pname )* $( , $lname )*)
                }
                Connection::Mysql(con) => {
                    con.write_query(mysql_query($( $pname, )* $( $lname, )*))
                }
            }
        }

        fn query_internal_with_transaction(
            mut transaction: Transaction,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> BoxFuture<(Transaction, WriteResult), Error> {
            match transaction {
                Transaction::Sqlite(ref mut transaction) => {
                    let con = transaction
                        .take()
                        .expect("should be Some before transaction ended");

                    sqlite_exec_query_with_transaction(con $( , $pname )* $( , $lname )*)
                        .map(move |(con, res)| {
                            (Transaction::Sqlite(Some(con)), res)
                        })
                        .boxify()
                }
                Transaction::Mysql(ref mut transaction) => {
                    let transaction = transaction
                        .take()
                        .expect("should be Some before transaction ended");
                    transaction
                        .write_query(mysql_query($( $pname, )* $( $lname, )*))
                        .map(|(tra, result)| (Transaction::Mysql(Some(tra)), result))
                        .boxify()
                },
            }
        }

        fn mysql_query($( $pname: & $ptype, )* $( $lname: & [ $ltype ], )*) -> String {
            $crate::_emit_mysql_lnames!($( $lname ),*);
            $crate::_write_mysql_query!($qtype, $mysql_q, $( $pname ),* $( >list $lname )*)
        }

        fn sqlite_exec_query(
            multithread_con: Arc<SqliteMultithreaded>,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> BoxFuture<WriteResult, Error> {
            $crate::_prepare_sqlite_params!(
                params,
                $( $pname ),*
                $( >list $lname )*
            );

            lazy(move || -> SqliteResult<WriteResult> {
                let con = multithread_con.get_sqlite_guard();

                let mut stmt = sqlite_statement(&con  $( , $lname )*)?;

                let mut param_refs: Vec<(&str, &ToSqliteValue)> = Vec::new();
                for param in &params {
                    param_refs.push((&param.0, &param.1));
                }

                let res = stmt.execute_named(param_refs.as_ref())?;

                Ok(WriteResult::new(
                    Some(con.last_insert_rowid() as u64),
                    res as u64,
                ))
            })
                .from_err()
                .boxify()
        }

        fn sqlite_exec_query_with_transaction(
            transaction: SqliteConnectionGuard,
            $( $pname: & $ptype, )*
            $( $lname: & [ $ltype ], )*
        ) -> BoxFuture<(SqliteConnectionGuard, WriteResult), Error> {
            $crate::_prepare_sqlite_params!(
                params,
                $( $pname ),*
                $( >list $lname )*
            );

            lazy(move || -> SqliteResult<(SqliteConnectionGuard, WriteResult)> {
                let res = {
                    let mut stmt = sqlite_statement(&transaction  $( , $lname )*)?;

                    let mut param_refs: Vec<(&str, &ToSqliteValue)> = Vec::new();
                    for param in &params {
                        param_refs.push((&param.0, &param.1));
                    }

                    stmt.execute_named(param_refs.as_ref())?
                };

                let res = WriteResult::new(
                    Some(transaction.last_insert_rowid() as u64),
                    res as u64,
                );

                Ok((transaction, res))
            })
                .from_err()
                .boxify()
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
            $( $pname = concat!(":", stringify!($pname)), )*
        )
    };

    (insert_or_ignore, $q:expr, $( $pname:ident ),* $( >list $lname:ident )*) => {
        format!(
            $q,
            insert_or_ignore = "INSERT OR IGNORE",
            $( $pname = concat!(":", stringify!($pname)), )*
            $( $lname = $lname, )*
        )
    };

    (none, $q:expr, values: $values:expr, $( $pname:ident ),*) => {
        format!(
            $q,
            values = $values,
            $( $pname = concat!(":", stringify!($pname)), )*
        )
    };

    (none, $q:expr, $( $pname:ident ),* $( >list $lname:ident )*) => {
        format!(
            $q,
            $( $pname = concat!(":", stringify!($pname)), )*
            $( $lname = $lname, )*
        )
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! _sqlite_named_params {
    ($params:ident, $tup:ident $( , $vname:ident )*) => (
        $crate::_sqlite_named_params!(@step 0 $params, $tup $( , $vname )*);
    );

    (@step 0 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(0 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 1 $params, $tup $( , $vnames )*);
    );

    (@step 1 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(1 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 2 $params, $tup $( , $vnames )*);
    );

    (@step 2 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(2 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 3 $params, $tup $( , $vnames )*);
    );

    (@step 3 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(3 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 4 $params, $tup $( , $vnames )*);
    );

    (@step 4 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(4 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 5 $params, $tup $( , $vnames )*);
    );

    (@step 5 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(5 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 6 $params, $tup $( , $vnames )*);
    );

    (@step 5 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(5 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 6 $params, $tup $( , $vnames )*);
    );

    (@step 6 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(6 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 7 $params, $tup $( , $vnames )*);
    );

    (@step 7 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(7 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 8 $params, $tup $( , $vnames )*);
    );

    (@step 8 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(8 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 9 $params, $tup $( , $vnames )*);
    );

    (@step 9 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(9 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 10 $params, $tup $( , $vnames )*);
    );

    (@step 10 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(10 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 11 $params, $tup $( , $vnames )*);
    );

    (@step 11 $params:ident, $tup:ident, $vname:ident $( , $vnames:ident )*) => (
        $crate::_sqlite_named_params_impl!(11 $params, $tup, $vname);
        $crate::_sqlite_named_params!(@step 12 $params, $tup $( , $vnames )*);
    );

    (@step $step:tt $params:ident, $tup:ident) => ();
}

#[macro_export]
#[doc(hidden)]
macro_rules! _sqlite_named_params_impl {
    ($step:tt $params:ident, $tup:ident, $vname:ident) => {
        $params.push((
            concat!(":", stringify!($vname)),
            ValueWrapper(ToValue::to_value(&$tup.$step)),
        ));
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! _append_to_mysql_values {
    ($values:ident, $tup:ident, $( $vtype:ty, )*) => (
        $crate::_append_to_mysql_values!(@step 0 $values, $tup, $( $vtype, )*);
    );

    (@step 0 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(0 $values, $tup);
        $crate::_append_to_mysql_values!(@step 1 $values, $tup, $( $vtypes, )*);
    );

    (@step 1 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(1 $values, $tup);
        $crate::_append_to_mysql_values!(@step 2 $values, $tup, $( $vtypes, )*);
    );

    (@step 2 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(2 $values, $tup);
        $crate::_append_to_mysql_values!(@step 3 $values, $tup, $( $vtypes, )*);
    );

    (@step 3 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(3 $values, $tup);
        $crate::_append_to_mysql_values!(@step 4 $values, $tup, $( $vtypes, )*);
    );

    (@step 4 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(4 $values, $tup);
        $crate::_append_to_mysql_values!(@step 5 $values, $tup, $( $vtypes, )*);
    );

    (@step 5 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(5 $values, $tup);
        $crate::_append_to_mysql_values!(@step 6 $values, $tup, $( $vtypes, )*);
    );

    (@step 6 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(6 $values, $tup);
        $crate::_append_to_mysql_values!(@step 7 $values, $tup, $( $vtypes, )*);
    );

    (@step 7 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(7 $values, $tup);
        $crate::_append_to_mysql_values!(@step 8 $values, $tup, $( $vtypes, )*);
    );

    (@step 8 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(8 $values, $tup);
        $crate::_append_to_mysql_values!(@step 9 $values, $tup, $( $vtypes, )*);
    );

    (@step 9 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(9 $values, $tup);
        $crate::_append_to_mysql_values!(@step 10 $values, $tup, $( $vtypes, )*);
    );

    (@step 10 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(10 $values, $tup);
        $crate::_append_to_mysql_values!(@step 11 $values, $tup, $( $vtypes, )*);
    );

    (@step 11 $values:ident, $tup:ident, $vtype:ty, $( $vtypes:ty, )+) => (
        $crate::_append_to_mysql_values_impl!(11 $values, $tup);
        $crate::_append_to_mysql_values!(@step 12 $values, $tup, $( $vtypes, )*);
    );

    (@step $step:tt $values:ident, $tup:ident, $vtype:ty,) => (
        write!(&mut $values, "{}", $tup.$step.to_value().as_sql(false)).unwrap();
    );
}

#[macro_export]
#[doc(hidden)]
macro_rules! _append_to_mysql_values_impl {
    ($step:tt $values:ident, $tup:ident) => {
        write!(&mut $values, "{}, ", $tup.$step.to_value().as_sql(false)).unwrap();
    };
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
            (format!(":{}", stringify!($pname)), ValueWrapper(ToValue::to_value($pname)))
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
