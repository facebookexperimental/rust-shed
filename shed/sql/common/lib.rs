/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Contains basic definitions for the sql crate and for any crate that wish
//! to implement traits to be used with the sql's queries macro.

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

pub mod mysql;
pub mod sqlite;
pub mod transaction;

use std::fmt;
use std::fmt::Debug;

use mysql_async::Value;
use mysql_async::prelude::FromValue;
use vec1::Vec1;

/// Struct to store a set of write, read and read-only connections for a shard.
#[derive(Clone)]
pub struct SqlConnections {
    /// Write connection to the master
    pub write_connection: Connection,
    /// Read connection
    pub read_connection: Connection,
    /// Read master connection
    pub read_master_connection: Connection,
}

impl SqlConnections {
    /// Create SqlConnections from a single connection.
    pub fn new_single(connection: Connection) -> Self {
        Self {
            write_connection: connection.clone(),
            read_connection: connection.clone(),
            read_master_connection: connection,
        }
    }
}

/// Struct to store a set of write, read and read-only connections for multiple shards.
#[derive(Clone)]
pub struct SqlShardedConnections {
    /// Write connections to the master for each shard
    pub write_connections: Vec1<Connection>,
    /// Read connections for each shard
    pub read_connections: Vec1<Connection>,
    /// Read master connections for each shard
    pub read_master_connections: Vec1<Connection>,
}

impl From<Vec1<SqlConnections>> for SqlShardedConnections {
    fn from(shard_connections: Vec1<SqlConnections>) -> Self {
        let (head, last) = shard_connections.split_off_last();
        let (write_connections, read_connections, read_master_connections) =
            itertools::multiunzip(head.into_iter().map(|conn| {
                (
                    conn.write_connection,
                    conn.read_connection,
                    conn.read_master_connection,
                )
            }));

        Self {
            read_connections: Vec1::from_vec_push(read_connections, last.read_connection),
            read_master_connections: Vec1::from_vec_push(
                read_master_connections,
                last.read_master_connection,
            ),
            write_connections: Vec1::from_vec_push(write_connections, last.write_connection),
        }
    }
}

/// Enum that generalizes over connections to Sqlite and MyRouter.
#[derive(Clone)]
pub enum Connection {
    /// Sqlite lets you use this crate with rusqlite connections such as in memory or on disk Sqlite
    /// databases, both useful in case of testing or local sql db use cases.
    Sqlite(sqlite::SqliteMultithreaded),
    /// A variant used for Meta's internal Mysql client connection factory.
    Mysql(mysql::Connection),
    /// For use in external Mysql DBs
    OssMysql(mysql::OssConnection),
}

impl From<sqlite::SqliteMultithreaded> for Connection {
    fn from(con: sqlite::SqliteMultithreaded) -> Self {
        Connection::Sqlite(con)
    }
}

impl From<mysql::Connection> for Connection {
    fn from(conn: mysql::Connection) -> Self {
        Connection::Mysql(conn)
    }
}

impl From<mysql::OssConnection> for Connection {
    fn from(conn: mysql::OssConnection) -> Self {
        Connection::OssMysql(conn)
    }
}

impl Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Connection::Sqlite(..) => write!(f, "Sqlite"),
            Connection::Mysql(..) => write!(f, "Meta internal Mysql client"),
            Connection::OssMysql(..) => write!(f, "AWS compatible Mysql client"),
        }
    }
}

/// Value returned from a `write` type of query
#[derive(Debug)]
pub struct WriteResult {
    last_insert_id: Option<u64>,
    affected_rows: u64,
}

impl WriteResult {
    /// Method made public for access from inside macros, you probably don't want to use it.
    pub fn new(last_insert_id: Option<u64>, affected_rows: u64) -> Self {
        WriteResult {
            last_insert_id,
            affected_rows,
        }
    }

    /// Return the id of last inserted row if any.
    pub fn last_insert_id(&self) -> Option<u64> {
        self.last_insert_id
    }

    /// Return the id of last inserted row if any, as any type that is
    /// convertable from a MySQL Value.
    pub fn last_insert_id_as<T: FromValue>(&self) -> Option<T> {
        self.last_insert_id.map(|id| T::from_value(Value::UInt(id)))
    }

    /// Return number of rows affected by the `write` query
    pub fn affected_rows(&self) -> u64 {
        self.affected_rows
    }
}

/// Telemetry returned after a query is executed or transaction is committed.
#[derive(Debug, Clone)]
pub enum QueryTelemetry {
    #[cfg(fbcode_build)]
    /// Internal MySQL
    MySQL(mysql::MysqlQueryTelemetry),
    /// OSS MySQL
    OssMySQL(mysql::OssQueryTelemetry),
}

#[cfg(fbcode_build)]
impl From<mysql::MysqlQueryTelemetry> for QueryTelemetry {
    fn from(telemetry: mysql::MysqlQueryTelemetry) -> Self {
        QueryTelemetry::MySQL(telemetry)
    }
}
