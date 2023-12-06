/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::sync::Arc;
use std::sync::RwLock;

use anyhow::bail;
use anyhow::Result;
use bytes::Bytes;
use tokio::sync::watch::channel;
use tokio::sync::watch::Receiver;
use tokio::sync::watch::Sender;

use crate::Entity;
use crate::ModificationTime;

// Type-erasure trick. I don't actually care about T for RegisteredConfigEntity,
/// so hide it via a trait object
pub(crate) trait Refreshable {
    fn get_path(&self) -> &str;
    fn refresh(&self, entity: Entity) -> Result<bool>;
}

/// The type contained in a `ConfigHandle` when it's obtained from a `ConfigStore`
pub(crate) struct RegisteredConfigEntity<T> {
    contents: RwLock<CachedConfigEntity>,
    path: String,
    deserializer: fn(Bytes) -> Result<T>,
    update_sender: RwLock<Sender<Arc<T>>>,
    update_receiver: RwLock<Receiver<Arc<T>>>,
}

struct CachedConfigEntity {
    mod_time: ModificationTime,
    version: String,
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
        let (update_sender, update_receiver) = channel(contents);

        Ok(Self {
            contents: RwLock::new(CachedConfigEntity { mod_time, version }),
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
            if update_sender.send(contents).is_err() {
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
                };
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }
}
