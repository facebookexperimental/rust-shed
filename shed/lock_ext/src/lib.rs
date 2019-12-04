/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

//! Crate extending functionalities of [std::sync]

use parking_lot::Mutex as ParkingLotMutex;
use std::sync::{Mutex, RwLock};

/// Extend functionality of [std::sync::Mutex]
///
/// # Example
/// ```
/// # use std::sync::Mutex;
/// # use lock_ext::LockExt;
/// let lock = Mutex::new(Vec::new());
/// lock.with(|value| value.push("hello"));
/// let hello = lock.with(|value| value.get(0).unwrap().to_owned());
/// # assert_eq!(&hello, &"hello");
/// ```
pub trait LockExt {
    /// Value that is being held inside the lock
    type Value;

    /// The passed `scope` function will be called with the lock being held
    /// and the locked value will be accessible inside the `scope` as `&mut`
    fn with<Scope, Out>(&self, scope: Scope) -> Out
    where
        Scope: FnOnce(&mut Self::Value) -> Out;
}

impl<V> LockExt for Mutex<V> {
    type Value = V;

    fn with<Scope, Out>(&self, scope: Scope) -> Out
    where
        Scope: FnOnce(&mut Self::Value) -> Out,
    {
        let mut value = self.lock().expect("lock poisoned");
        scope(&mut *value)
    }
}

impl<V> LockExt for ParkingLotMutex<V> {
    type Value = V;

    fn with<Scope, Out>(&self, scope: Scope) -> Out
    where
        Scope: FnOnce(&mut Self::Value) -> Out,
    {
        let mut value = self.lock();
        scope(&mut *value)
    }
}

/// Extend functionality of [std::sync::RwLock]
///
/// # Example
/// ```
/// # use std::sync::RwLock;
/// # use lock_ext::RwLockExt;
/// let lock = RwLock::new(Vec::new());
/// lock.with_write(|value| value.push("hello"));
/// let hello = lock.with_read(|value| value.get(0).unwrap().to_owned());
/// # assert_eq!(&hello, &"hello");
/// ```
pub trait RwLockExt {
    /// Value that is being held inside the lock
    type Value;

    /// The passed `scope` function will be called with the read lock being held
    /// and the locked value will be accessible inside the `scope` as `&`
    fn with_read<Scope, Out>(&self, scope: Scope) -> Out
    where
        Scope: FnOnce(&Self::Value) -> Out;

    /// The passed `scope` function will be called with the write lock being held
    /// and the locked value will be accessible inside the `scope` as `&mut`
    fn with_write<Scope, Out>(&self, scope: Scope) -> Out
    where
        Scope: FnOnce(&mut Self::Value) -> Out;
}

impl<V> RwLockExt for RwLock<V> {
    type Value = V;

    fn with_read<Scope, Out>(&self, scope: Scope) -> Out
    where
        Scope: FnOnce(&Self::Value) -> Out,
    {
        let value = self.read().expect("lock poisoned");
        scope(&*value)
    }

    fn with_write<Scope, Out>(&self, scope: Scope) -> Out
    where
        Scope: FnOnce(&mut Self::Value) -> Out,
    {
        let mut value = self.write().expect("lock poisoned");
        scope(&mut *value)
    }
}

#[cfg(test)]
mod test {
    use super::{LockExt, RwLockExt};
    use std::sync::{Arc, Mutex, RwLock};

    #[test]
    fn simple() {
        let vs = Arc::new(Mutex::new(Vec::new()));
        assert_eq!(vs.with(|vs| vs.len()), 0);
        vs.with(|vs| vs.push("test"));
        assert_eq!(vs.with(|vs| vs.pop()), Some("test"));
        assert_eq!(vs.with(|vs| vs.len()), 0);
    }

    #[test]
    fn rwlock() {
        let vs = Arc::new(RwLock::new(Vec::new()));
        assert_eq!(vs.with_read(|vs| vs.len()), 0);
        vs.with_write(|vs| vs.push("test"));
        assert_eq!(vs.with_write(|vs| vs.pop()), Some("test"));
        assert_eq!(vs.with_read(|vs| vs.len()), 0);
    }
}
