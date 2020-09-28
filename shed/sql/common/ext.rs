/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Utilities for interacting with Connection.

use crate::{deprecated_mysql::MysqlConnection, error::from_failure, mysql, Connection};
use anyhow::Error;
use futures::{future::FutureExt as _, future::TryFutureExt};
use futures_ext::{BoxFuture, FutureExt};
use futures_old::{
    future::{ok, Future},
    sync::oneshot,
};

/// Extension trait for adding additional functionality to connections.
pub trait ConnectionExt {
    /// Returns the replication lag for a connection.
    /// Only implemented for MySQL connections at this time.
    /// For Sqlite connections it will always return None.
    fn show_replica_lag_secs(&self) -> BoxFuture<Option<u64>, Error>;
}

impl ConnectionExt for Connection {
    fn show_replica_lag_secs(&self) -> BoxFuture<Option<u64>, Error> {
        match self {
            Connection::Sqlite(_) => ok(None).boxify(),
            Connection::DeprecatedMysql(ref con) => con.show_replica_lag_secs(),
            Connection::Mysql(ref con) => con.show_replica_lag_secs(),
        }
    }
}

trait MysqlConnectionExt {
    fn show_replica_lag_secs(&self) -> BoxFuture<Option<u64>, Error>;
}

impl<T: MysqlConnection> MysqlConnectionExt for T {
    fn show_replica_lag_secs(&self) -> BoxFuture<Option<u64>, Error> {
        let (tx, rx) = oneshot::channel();
        self.query(
            "show slave status".to_owned(),
            Box::new(|query_result| {
                query_result
                    .reduce(None, |val, row| {
                        val.or_else(|| row.get("Seconds_Behind_Master"))
                    })
                    .boxed()
                    .compat()
                    .map(|(query_result, result)| {
                        assert!(
                            tx.send(result).is_ok(),
                            "Unexpectedly, receiver of the channel has been dropped"
                        );
                        query_result
                    })
                    .map_err(from_failure)
                    .boxify()
            }),
        )
        .and_then(|()| rx.map_err(|err| err.into()))
        .boxify()
    }
}

impl MysqlConnectionExt for mysql::Connection {
    fn show_replica_lag_secs(&self) -> BoxFuture<Option<u64>, Error> {
        let this = self.clone();
        async move { this.get_replica_lag_secs().map_err(Error::from).await }
            .boxed()
            .compat()
            .boxify()
    }
}
