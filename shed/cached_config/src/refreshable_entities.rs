/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::sync::Arc;
use std::sync::RwLock;

use anyhow::Result;
use anyhow::bail;
use bytes::Bytes;
use tokio::sync::watch::Receiver;
use tokio::sync::watch::Sender;
use tokio::sync::watch::channel;

use crate::Entity;
use crate::ModificationTime;
use crate::handle::ConfigVersionInfo;

// Type-erasure trick. I don't actually care about T for RegisteredConfigEntity,
/// so hide it via a trait object
pub(crate) trait Refreshable {
    fn get_path(&self) -> &str;
    fn refresh(&self, entity: Entity) -> Result<bool>;
}

/// The type contained in a `ConfigHandle` when it's obtained from a `ConfigStore`
pub(crate) struct RegisteredConfigEntity<T> {
    contents: RwLock<CachedConfigEntity<T>>,
    path: String,
    deserializer: fn(Bytes) -> Result<T>,
    update_sender: RwLock<Sender<Arc<T>>>,
    update_receiver: RwLock<Receiver<Arc<T>>>,
}

/// A single config snapshot: the deserialized contents together with the
/// version metadata they were parsed from. All fields are committed in the
/// same write-lock critical section during `refresh`, so readers holding the
/// read lock always observe contents and version that correspond.
struct CachedConfigEntity<T> {
    mod_time: ModificationTime,
    version: String,
    contents: Arc<T>,
}

impl<T> RegisteredConfigEntity<T>
where
    T: Send + Sync + 'static,
{
    pub(crate) fn new(
        path: String,
        entity: Entity,
        deserializer: fn(Bytes) -> Result<T>,
    ) -> Result<Self> {
        let Entity {
            mod_time,
            version,
            contents,
        } = entity;
        let contents = Arc::new(deserializer(contents.unwrap_or_else(Bytes::new))?);
        let (update_sender, update_receiver) = channel(contents.clone());

        Ok(Self {
            contents: RwLock::new(CachedConfigEntity {
                mod_time,
                version,
                contents,
            }),
            path,
            deserializer,
            update_sender: RwLock::new(update_sender),
            update_receiver: RwLock::new(update_receiver),
        })
    }

    pub(crate) fn get(&self) -> Arc<T> {
        self.update_receiver
            .read()
            .expect("lock poisoned")
            .borrow()
            .clone()
    }

    /// Get the current contents together with the version metadata they were
    /// parsed from. Both are read under a single read-lock acquisition, so
    /// they are guaranteed to correspond to the same config snapshot even if
    /// a refresh is in flight.
    pub(crate) fn get_with_version(&self) -> (Arc<T>, ConfigVersionInfo) {
        let locked = self.contents.read().expect("lock poisoned");
        (
            locked.contents.clone(),
            ConfigVersionInfo {
                version: locked.version.clone(),
                mod_time: locked.mod_time.clone(),
            },
        )
    }

    pub(crate) fn update_receiver(&self) -> Receiver<Arc<T>> {
        self.update_receiver.read().expect("lock poisoned").clone()
    }
}

impl<T> Refreshable for RegisteredConfigEntity<T>
where
    T: Send + Sync + 'static,
{
    fn get_path(&self) -> &str {
        &self.path
    }

    fn refresh(&self, entity: Entity) -> Result<bool> {
        let has_changed = {
            let locked = self.contents.read().expect("lock poisoned");
            entity.mod_time != locked.mod_time || entity.version != locked.version
        };

        if has_changed {
            let contents = Arc::new((self.deserializer)(entity.contents.unwrap_or_default())?);
            let update_sender = self.update_sender.write().expect("lock poisoned");
            // Deliberate ordering: the watch channel is updated before the
            // snapshot lock below is committed, so watchers/get() can briefly
            // see newer contents than get_with_version(). Each accessor family
            // is self-consistent; do not "fix" the order.
            if update_sender.send(contents.clone()).is_err() {
                bail!(
                    "No subscriber for config updates at path {}",
                    self.get_path()
                )
            }
            {
                let mut locked = self.contents.write().expect("lock poisoned");
                *locked = CachedConfigEntity {
                    mod_time: entity.mod_time,
                    version: entity.version,
                    contents,
                };
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }
}
