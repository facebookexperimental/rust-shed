/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use bytes::Bytes;
use tokio::sync::watch::Receiver;
use tokio::sync::watch::Sender;
use tokio::sync::watch::channel;

use crate::Entity;
use crate::handle::ConfigVersionInfo;

// Type-erasure trick. I don't actually care about T for RegisteredConfigEntity,
/// so hide it via a trait object
pub(crate) trait Refreshable {
    fn get_path(&self) -> &str;
    fn refresh(&self, entity: Entity) -> Result<bool>;
}

/// The type contained in a `ConfigHandle` when it's obtained from a `ConfigStore`
pub(crate) struct RegisteredConfigEntity<T> {
    path: String,
    deserializer: fn(Bytes) -> Result<T>,
    update_sender: Mutex<Sender<Snapshot<T>>>,
    // Keeps config handles `RefUnwindSafe` when tokio uses `parking_lot` (as in
    // OSS builds). A snapshot is only ever replaced whole, so a panic cannot
    // leave a reader with a partial one.
    update_receiver: AssertUnwindSafe<Receiver<Snapshot<T>>>,
}

/// A config's contents with the version metadata they were parsed from. Every
/// reader, watchers included, sees the two together.
pub(crate) struct Snapshot<T> {
    pub(crate) contents: Arc<T>,
    pub(crate) version_info: ConfigVersionInfo,
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
        let (update_sender, update_receiver) = channel(Snapshot {
            contents: Arc::new(deserializer(contents.unwrap_or_else(Bytes::new))?),
            version_info: ConfigVersionInfo { version, mod_time },
        });

        Ok(Self {
            path,
            deserializer,
            update_sender: Mutex::new(update_sender),
            update_receiver: AssertUnwindSafe(update_receiver),
        })
    }

    pub(crate) fn get(&self) -> Arc<T> {
        self.update_receiver.borrow().contents.clone()
    }

    /// Get the current contents together with the version metadata they were
    /// parsed from.
    pub(crate) fn get_with_version(&self) -> (Arc<T>, ConfigVersionInfo) {
        let snapshot = self.update_receiver.borrow();
        (snapshot.contents.clone(), snapshot.version_info.clone())
    }

    pub(crate) fn update_receiver(&self) -> Receiver<Snapshot<T>> {
        self.update_receiver.clone()
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
        // The check and the send are under one lock, so refreshes that overlap
        // (the updater thread and `force_update_configs`) apply in turn.
        let update_sender = self.update_sender.lock().expect("lock poisoned");
        {
            let current = update_sender.borrow();
            if entity.mod_time == current.version_info.mod_time
                && entity.version == current.version_info.version
            {
                return Ok(false);
            }
        }

        let snapshot = Snapshot {
            contents: Arc::new((self.deserializer)(entity.contents.unwrap_or_default())?),
            version_info: ConfigVersionInfo {
                version: entity.version,
                mod_time: entity.mod_time,
            },
        };
        update_sender.send_replace(snapshot);
        Ok(true)
    }
}
