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

use lazy_static::lazy_static;
use rusqlite::Connection as SqliteConnection;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::Condvar;
use std::sync::Mutex;

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
pub struct SqliteMultithreaded {
    con: Arc<Mutex<Option<SqliteConnection>>>,
    condvar: Arc<Condvar>,
}

/// Returns a guard that grabs a lock and connection. Can be used instead of SqliteConnection
/// When guard is destroyed then connection is put back and threads that are waiting for it
/// are notified
pub struct SqliteConnectionGuard {
    m: Arc<Mutex<Option<SqliteConnection>>>,
    condvar: Arc<Condvar>,
    // drop() need to remove the connection, so use Option<...> here
    con: Option<SqliteConnection>,
}

impl SqliteConnectionGuard {
    fn new(
        m: Arc<Mutex<Option<SqliteConnection>>>,
        condvar: Arc<Condvar>,
    ) -> SqliteConnectionGuard {
        let _global_lock =
            CONN_CONDVAR.wait_while(CONN_LOCK.lock().expect("lock poisoned"), |allowed| {
                if *allowed {
                    *allowed = false;
                    false
                } else {
                    true
                }
            });
        let con = {
            let mut mutexguard = condvar
                .wait_while(m.lock().expect("poisoned lock"), |con| con.is_none())
                .expect("poisoned lock");

            mutexguard.take().expect("connection should not be empty")
        };

        SqliteConnectionGuard {
            m,
            condvar,
            con: Some(con),
        }
    }
}

impl Deref for SqliteConnectionGuard {
    type Target = SqliteConnection;

    fn deref(&self) -> &Self::Target {
        self.con
            .as_ref()
            .expect("invariant violation - deref called after drop()")
    }
}

impl Drop for SqliteConnectionGuard {
    fn drop(&mut self) {
        *(CONN_LOCK.lock().expect("lock poisoned")) = true;
        let mut locked_m = self.m.lock().expect("poisoned lock");
        locked_m.get_or_insert(self.con.take().unwrap());
        // notify others that wait for this connection
        self.condvar.notify_one();
        CONN_CONDVAR.notify_one();
    }
}

impl SqliteMultithreaded {
    /// Create a new instance wrapping the provided sqlite connection.
    pub fn new(con: SqliteConnection) -> Self {
        Self {
            con: Arc::new(Mutex::new(Some(con))),
            condvar: Arc::new(Condvar::new()),
        }
    }

    /// Returns a guard that grabs a lock and connection.
    /// When guard is destroyed then connection is put back and threads that are waiting for it
    /// are notified
    /// NOTE: it will block any other `get_sqlite_guard()` calls. So you shouldn't be async i.e.
    /// if you have a future that calls `get_sqlite_guard()` then it shouldn't return NotReady
    /// because it can cause a deadlock if another future will try to grab get_sqlite_guard
    pub fn get_sqlite_guard(&self) -> SqliteConnectionGuard {
        SqliteConnectionGuard::new(self.con.clone(), self.condvar.clone())
    }
}
