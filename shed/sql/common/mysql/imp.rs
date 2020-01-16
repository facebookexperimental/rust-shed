/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use anyhow::Error;
use futures::future::Future;
use futures_ext::{BoxFuture, FutureExt};
use mysql_async::{prelude::Queryable, Conn, Transaction};

use super::{BoxMysqlTransaction, MysqlTransaction, TraQueryProcess};
use crate::error::from_failure;

impl MysqlTransaction for Transaction<Conn> {
    fn query(
        self: Box<Self>,
        query: String,
        process: TraQueryProcess,
    ) -> BoxFuture<BoxMysqlTransaction, Error> {
        (*self)
            .query(query)
            .map_err(from_failure)
            .and_then(move |query_result| process(query_result))
            .and_then(|query_result| query_result.drop_result().map_err(from_failure))
            .map(move |transaction| -> BoxMysqlTransaction { Box::new(transaction) })
            .boxify()
    }

    fn commit(self: Box<Self>) -> BoxFuture<(), Error> {
        (*self)
            .commit()
            .and_then(|conn| conn.disconnect())
            .map_err(from_failure)
            .boxify()
    }

    fn rollback(self: Box<Self>) -> BoxFuture<(), Error> {
        (*self)
            .rollback()
            .and_then(|conn| conn.disconnect())
            .map_err(from_failure)
            .boxify()
    }
}
