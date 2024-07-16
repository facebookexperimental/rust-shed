/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module hides the implementation details of the Facebook Mysql client library
//! and provides API that is used in sql crate.

use std::sync::Arc;

use anyhow::Error;
use futures_stats::futures03::TimedFutureExt;
use mysql_async::prelude::Queryable;
use mysql_async::Conn as MysqlConnection;
use mysql_async::Pool;
use mysql_async::QueryResult as MysqlQueryResult;
use mysql_async::TextProtocol;
use mysql_async::Transaction;
use mysql_async::TxOpts;
use stats::prelude::*;
use time_ext::DurationExt;

use crate::mysql::ConnectionStats;
use crate::mysql::WriteResult;

type QueryResult<'a> = MysqlQueryResult<'a, 'static, TextProtocol>;

/// OssConnection is a wrapper around a MySQL async Pool
/// It provides read/write query and begin transaction API.
#[derive(Clone)]
pub struct OssConnection {
    /// Connection pool
    pub pool: Pool,
    /// Stats struct for logging performance
    pub stats: Arc<ConnectionStats>,
}

impl OssConnection {
    /// Creates OssConnection from a Pool object
    pub fn new(pool: Pool, stats: Arc<ConnectionStats>) -> Self {
        Self { pool, stats }
    }

    /// Checks out a connection from the pool while collecting stats
    pub async fn get_conn_counted(
        pool: Pool,
        stats: &ConnectionStats,
    ) -> Result<MysqlConnection, mysql_async::Error> {
        let (st, conn) = pool.get_conn().timed().await;
        stats
            .get_connection_ms
            .add_value(st.completion_time.as_millis_unchecked() as i64);
        conn
    }

    /// Executes a given query and returns the result while collecting stats
    pub async fn raw_query_counted<'a>(
        conn: &'a mut MysqlConnection,
        stats: &ConnectionStats,
        query: &'a str,
    ) -> Result<QueryResult<'a>, mysql_async::Error> {
        let (st, result) = conn.query_iter(query).timed().await;
        stats
            .raw_query_ms
            .add_value(st.completion_time.as_millis_unchecked() as i64);
        result
    }

    /// Performs a given query and returns the result as a QueryResult
    pub async fn read_query<'a>(
        &self,
        conn: &'a mut MysqlConnection,
        query: &'a str,
    ) -> Result<QueryResult<'a>, mysql_async::Error> {
        OssConnection::raw_query_counted(conn, &self.stats, query).await
    }

    /// Performs a given query and returns the write result.
    pub async fn write_query(&self, query: String) -> Result<WriteResult, Error> {
        let mut conn = OssConnection::get_conn_counted(self.pool.clone(), &self.stats).await?;
        let result = OssConnection::raw_query_counted(&mut conn, &self.stats, &query).await?;

        let last_insert_id = result.last_insert_id().unwrap_or(0);
        let rows_affected = result.affected_rows();
        Ok(WriteResult::new(last_insert_id, rows_affected))
    }

    /// Begins transaction and returns Transaction object.
    pub async fn begin_transaction(&self, tx_opts: TxOpts) -> Result<Transaction<'static>, Error> {
        let tr = self.pool.start_transaction(tx_opts).await?;

        Ok(tr)
    }
}
