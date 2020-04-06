/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Error;
use futures::{compat::Future01CompatExt, future::FutureExt, future::TryFutureExt};
use futures_ext::{BoxFuture, FutureExt as _};
use mysql_async::{prelude::Queryable, Conn, Transaction};

use super::{BoxMysqlTransaction, MysqlTransaction, TraQueryProcess};

impl MysqlTransaction for Transaction<Conn> {
    fn query(
        self: Box<Self>,
        query: String,
        process: TraQueryProcess,
    ) -> BoxFuture<BoxMysqlTransaction, Error> {
        async move {
            let query_result = (*self).query(query).await?;
            let query_result = process(query_result).compat().await?;
            let transaction = query_result.drop_result().await?;
            Ok(Box::new(transaction) as BoxMysqlTransaction)
        }
        .boxed()
        .compat()
        .boxify()
    }

    fn commit(self: Box<Self>) -> BoxFuture<(), Error> {
        async move {
            let conn = (*self).commit().await?;
            conn.disconnect().await?;
            Ok(())
        }
        .boxed()
        .compat()
        .boxify()
    }

    fn rollback(self: Box<Self>) -> BoxFuture<(), Error> {
        async move {
            let conn = (*self).rollback().await?;
            conn.disconnect().await?;
            Ok(())
        }
        .boxed()
        .compat()
        .boxify()
    }
}
