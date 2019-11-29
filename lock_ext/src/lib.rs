/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use parking_lot::Mutex as ParkingLotMutex;
use std::sync::{Mutex, RwLock};

pub trait LockExt {
    type Value;

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

pub trait RwLockExt {
    type Value;

    fn with_read<Scope, Out>(&self, scope: Scope) -> Out
    where
        Scope: FnOnce(&Self::Value) -> Out;

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
