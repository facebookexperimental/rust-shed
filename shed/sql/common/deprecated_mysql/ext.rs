/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Error;
use futures::{future::FutureExt as _, future::TryFutureExt};
use futures_ext::{BoxFuture, FutureExt};
use futures_old::{
    future::{ok, Future as OldFuture},
    sync::oneshot,
};
use mysql_async::{prelude::FromRow, FromRowError};

use super::{BoxMysqlConnection, BoxMysqlTransaction, QueryProcess, QueryResult};
use crate::error::from_failure;
use crate::WriteResult;

type MyBoxFuture<T> = BoxFuture<T, Error>;

/// An extension trait that provides useful methods for Boxed version of
/// [crate::deprecated_mysql::MysqlConnection]. Cannot be provided directly
/// on the trait, because the usage of generics in the methods would prevent
/// from creating a trait object.
pub trait MysqlConnectionExt {
    /// Perform the string query and collect the result.
    /// See [::mysql_async::QueryResult::collect]
    fn read_query<R: FromRow + Send + 'static>(&self, query: String) -> MyBoxFuture<Vec<R>>;
    /// Perform the string query and try to collect the result.
    /// See [::mysql_async::QueryResult::try_collect]
    fn try_read_query<R: FromRow + Send + 'static>(
        &self,
        query: String,
    ) -> MyBoxFuture<Vec<Result<R, FromRowError>>>;
    /// Perform the string query and return some metadata.
    fn write_query(&self, query: String) -> MyBoxFuture<WriteResult>;
}

impl MysqlConnectionExt for BoxMysqlConnection {
    fn read_query<R: FromRow + Send + 'static>(&self, query: String) -> MyBoxFuture<Vec<R>> {
        helper_query_process(
            |process| self.query(query, process),
            |query_result| {
                query_result
                    .collect()
                    .boxed()
                    .compat()
                    .map_err(from_failure)
            },
        )
        .map(|((), result)| result)
        .boxify()
    }

    fn try_read_query<R: FromRow + Send + 'static>(
        &self,
        query: String,
    ) -> MyBoxFuture<Vec<Result<R, FromRowError>>> {
        helper_query_process(
            |process| self.query(query, process),
            |query_result| {
                query_result
                    .try_collect()
                    .boxed()
                    .compat()
                    .map_err(from_failure)
            },
        )
        .map(|((), result)| result)
        .boxify()
    }

    fn write_query(&self, query: String) -> MyBoxFuture<WriteResult> {
        helper_query_process(
            |process| self.query(query, process),
            |query_result| {
                let write_result =
                    WriteResult::new(query_result.last_insert_id(), query_result.affected_rows());
                ok((query_result, write_result))
            },
        )
        .map(|((), result)| result)
        .boxify()
    }
}

/// An extension trait that provides useful methods for Boxed version of
/// [crate::deprecated_mysql::MysqlTransaction]. Cannot be provided directly
/// on the trait, because the usage of generics in the methods would prevent
/// from creating a trait object.
pub trait MysqlTransactionExt {
    /// Perform the string query and collect the result.
    /// See [::mysql_async::QueryResult::collect]
    fn read_query<R: FromRow + Send + 'static>(
        self,
        query: String,
    ) -> MyBoxFuture<(BoxMysqlTransaction, Vec<R>)>;
    /// Perform the string query and try to collect the result.
    /// See [::mysql_async::QueryResult::try_collect]
    fn try_read_query<R: FromRow + Send + 'static>(
        self,
        query: String,
    ) -> MyBoxFuture<(BoxMysqlTransaction, Vec<Result<R, FromRowError>>)>;
    /// Perform the string query and return some metadata.
    fn write_query(self, query: String) -> MyBoxFuture<(BoxMysqlTransaction, WriteResult)>;
}

impl MysqlTransactionExt for BoxMysqlTransaction {
    fn read_query<R: FromRow + Send + 'static>(
        self,
        query: String,
    ) -> MyBoxFuture<(BoxMysqlTransaction, Vec<R>)> {
        helper_query_process(
            |process| self.query(query, process),
            |query_result| {
                query_result
                    .collect()
                    .boxed()
                    .compat()
                    .map_err(from_failure)
            },
        )
    }

    fn try_read_query<R: FromRow + Send + 'static>(
        self,
        query: String,
    ) -> MyBoxFuture<(BoxMysqlTransaction, Vec<Result<R, FromRowError>>)> {
        helper_query_process(
            |process| self.query(query, process),
            |query_result| {
                query_result
                    .try_collect()
                    .boxed()
                    .compat()
                    .map_err(from_failure)
            },
        )
    }

    fn write_query(self, query: String) -> MyBoxFuture<(BoxMysqlTransaction, WriteResult)> {
        helper_query_process(
            |process| self.query(query, process),
            |query_result| {
                let write_result =
                    WriteResult::new(query_result.last_insert_id(), query_result.affected_rows());
                ok((query_result, write_result))
            },
        )
    }
}

fn helper_query_process<TQueryable, TProcess, TProcessFut, TProcessResult, TQueryResult>(
    query: impl FnOnce(QueryProcess<TQueryable>) -> MyBoxFuture<TProcessResult>,
    process: TProcess,
) -> MyBoxFuture<(TProcessResult, TQueryResult)>
where
    TQueryable: Send + 'static,
    TProcess: FnOnce(QueryResult<TQueryable>) -> TProcessFut + Send + 'static,
    TProcessFut:
        OldFuture<Item = (QueryResult<TQueryable>, TQueryResult), Error = Error> + Send + 'static,
    TProcessResult: Send + 'static,
    TQueryResult: Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    query(Box::new(move |query_result| {
        process(query_result)
            .map(|(query_result, result)| {
                let _ = tx.send(result);
                query_result
            })
            .boxify()
    }))
    .and_then(|process_result| {
        rx.map(|result| (process_result, result))
            .map_err(|err| err.into())
    })
    .boxify()
}
