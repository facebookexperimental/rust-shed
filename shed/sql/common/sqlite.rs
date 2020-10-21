/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module containing sqlite related structures and traits

#![allow(clippy::mutex_atomic)]

use futures::lock::{Mutex as AsyncMutex, MutexGuard};
use lazy_static::lazy_static;
use rusqlite::Connection as SqliteConnection;
use std::ops::Deref;
use std::sync::{Arc, Mutex as SyncMutex};

lazy_static! {
    /// Lock to ensure that only one connection is in use for writes at a time inside the process
    /// TODO: Remove this lock, and replace by better connection handling (as SQLite will get this right
    /// if we use a single connection to each file). See T59837828
    static ref CONN_LOCK: AsyncMutex<()> = AsyncMutex::new(());
}

impl crate::Connection {
    /// Given a `rusqlite::Connection` create a connection to Sqlite database that might be used
    /// by this crate.
    pub fn with_sqlite(con: SqliteConnection) -> Self {
        SqliteMultithreaded::new(con).into()
    }
}

type SqliteMultithreadedConnection = Arc<SyncMutex<Option<SqliteConnection>>>;

/// Wrapper around rusqlite connection that makes it fully thread safe (but not deadlock safe)
pub struct SqliteMultithreaded {
    con: SqliteMultithreadedConnection,
}

/// Returns a guard that grabs a lock and connection. Can be used instead of SqliteConnection
/// When guard is destroyed then connection is put back and threads that are waiting for it
/// are notified
pub struct SqliteConnectionGuard {
    m: SqliteMultithreadedConnection,
    global_guard: MutexGuard<'static, ()>,
    // drop() need to remove the connection, so use Option<...> here
    conn: Option<SqliteConnection>,
}

impl SqliteConnectionGuard {
    async fn lock(m: SqliteMultithreadedConnection) -> Self {
        let global_guard = CONN_LOCK.lock().await;
        let conn = m
            .lock()
            .expect("poisoned lock")
            .take()
            .expect("Connection should never be empty since we're under the global lock");
        Self {
            m,
            global_guard,
            conn: Some(conn),
        }
    }
}

impl Deref for SqliteConnectionGuard {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        &self
            .conn
            .as_ref()
            .expect("Connection shouldn't be empty, unless deref() is called after drop()")
    }
}

impl Drop for SqliteConnectionGuard {
    fn drop(&mut self) {
        self.m.lock().expect("poisoned lock").replace(
            self.conn
                .take()
                .expect("Connection shouldn't be empty, unless drop() is called twice"),
        );
        // We need to use the global_guard field somehow so that it's not considered dead code.
        // The guard will be unlocked when the SqliteConnectionGuard is dropped anyway.
        *self.global_guard
    }
}

impl SqliteMultithreaded {
    /// Create a new instance wrapping the provided sqlite connection.
    pub fn new(con: SqliteConnection) -> Self {
        Self {
            con: Arc::new(SyncMutex::new(Some(con))),
        }
    }

    /// Returns a guard that grabs a lock and connection.
    /// When guard is destroyed then connection is put back and tasks that are waiting for it
    /// are resumed
    pub async fn get_sqlite_guard(&self) -> SqliteConnectionGuard {
        SqliteConnectionGuard::lock(self.con.clone()).await
    }
}
