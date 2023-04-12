/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module containing sqlite related structures and traits

#![allow(clippy::mutex_atomic)]

use std::ops::Deref;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;

use lazy_static::lazy_static;
use rusqlite::Connection as SqliteConnection;

lazy_static! {
    /// Lock to ensure that only one connection is in use for writes at a time inside the process
    /// TODO: Remove this lock, and replace by better connection handling (as SQLite will get this right
    /// if we use a single connection to each file). See T59837828
    static ref CONN_LOCK: Mutex<bool> = Mutex::new(true);
    static ref CONN_CONDVAR: Condvar = Condvar::new();
}

impl crate::Connection {
    /// Given a `rusqlite::Connection` create a connection to Sqlite database that might be used
    /// by this crate.
    pub fn with_sqlite(con: SqliteConnection) -> Self {
        SqliteMultithreaded::new(con).into()
    }
}

/// Wrapper around rusqlite connection that makes it fully thread safe (but not deadlock safe)
#[derive(Clone)]
pub struct SqliteMultithreaded {
    inner: Arc<SqliteMultithreadedInner>,
}

/// Shared inner part of SqliteMultithreded plus any active connection guard.
pub struct SqliteMultithreadedInner {
    connection: Mutex<Option<SqliteConnection>>,
    condvar: Condvar,
}

/// Guard containing an active connection.
///
/// When this guard is destroyed, the connection is put back and threads that
/// are waiting for it are notified.
pub struct SqliteConnectionGuard {
    inner: Arc<SqliteMultithreadedInner>,
    // drop() needs to remove the connection, so use Option<...> here
    connection: Option<SqliteConnection>,
}

impl SqliteConnectionGuard {
    fn new(inner: Arc<SqliteMultithreadedInner>) -> SqliteConnectionGuard {
        let _global_lock =
            CONN_CONDVAR.wait_while(CONN_LOCK.lock().expect("lock poisoned"), |allowed| {
                if *allowed {
                    *allowed = false;
                    false
                } else {
                    true
                }
            });
        let connection = {
            let mut connection = inner
                .condvar
                .wait_while(inner.connection.lock().expect("poisoned lock"), |con| {
                    con.is_none()
                })
                .expect("poisoned lock");

            connection.take().expect("connection should not be empty")
        };

        SqliteConnectionGuard {
            inner,
            connection: Some(connection),
        }
    }
}

impl Deref for SqliteConnectionGuard {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        self.connection
            .as_ref()
            .expect("invariant violation - deref called after drop()")
    }
}

impl Drop for SqliteConnectionGuard {
    fn drop(&mut self) {
        *(CONN_LOCK.lock().expect("lock poisoned")) = true;
        let mut connection = self.inner.connection.lock().expect("poisoned lock");
        connection.get_or_insert(self.connection.take().unwrap());
        // notify others that wait for this connection
        self.inner.condvar.notify_one();
        CONN_CONDVAR.notify_one();
    }
}

impl SqliteMultithreaded {
    /// Create a new instance wrapping the provided sqlite connection.
    pub fn new(connection: SqliteConnection) -> Self {
        Self {
            inner: Arc::new(SqliteMultithreadedInner {
                connection: Mutex::new(Some(connection)),
                condvar: Condvar::new(),
            }),
        }
    }

    /// Returns a guard that grabs the sqlite connection.
    ///
    /// When guard is destroyed then connection is put back and threads that are waiting for it
    /// are notified.
    ///
    /// NOTE: This is a lock which will block any other `get_sqlite_guard()` calls, so you
    /// must not hold this over an await point as this may cause a deadlock.
    pub fn get_sqlite_guard(&self) -> SqliteConnectionGuard {
        SqliteConnectionGuard::new(self.inner.clone())
    }
}
