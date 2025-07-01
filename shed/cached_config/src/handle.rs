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

use anyhow::Context;
use anyhow::Result;
use anyhow::bail;
use serde::de::DeserializeOwned;
use serde_json::from_str;
use tokio::sync::watch::Receiver;

use crate::refreshable_entities::RegisteredConfigEntity;

/// A configuration handle, with self-refresh and wait-on-update if obtained
/// from a `ConfigStore`. If your type `T` implements `Default`, then this
/// will implement `Default` using a fixed config matching `T`'s default
#[derive(Clone)]
pub struct ConfigHandle<T> {
    inner: ConfigHandleImpl<T>,
}

/// A config update watcher that observes changes that are applied to the underlying config
/// and provides a mechanism to get notified on the next config change via wait-on-update
/// method. This type is defined only for configs that are backed by a config source and
/// not for static configs (e.g. static JSON files)
/// NOTE: The ConfigUpdateWatcher can only receive updates as long as its parent
/// ConfigHandle remains in scope. Once the corresponding ConfigHandle is dropped,
/// the ConfigUpdateWatcher can no longer receive updated configs.
pub struct ConfigUpdateWatcher<T> {
    update_receiver: Receiver<Arc<T>>,
}

impl<T> ConfigUpdateWatcher<T> {
    fn new(update_receiver: Receiver<Arc<T>>) -> Self {
        Self { update_receiver }
    }

    /// Method that returns only when the watcher observes a change
    /// in the underlying config. The return value is an Arc of the
    /// updated config. If the config updater has gone away when this
    /// method is called, it returns an error
    pub async fn wait_for_next(&mut self) -> Result<Arc<T>> {
        self.update_receiver
            .changed()
            .await
            .context("Error while waiting for the config updater to update the config")?;
        Ok(self.update_receiver.borrow().clone())
    }
}

// Enums have all their variants public, which needlessly exposes implementation
// details of the ConfigHandle, that is why this enum is wrapped in a struct.
#[derive(Clone)]
enum ConfigHandleImpl<T> {
    /// Config is obtained from a `ConfigStore`, and kept up to date
    Registered(Arc<RegisteredConfigEntity<T>>),
    /// Config is fixed. Obtained via `from_json`, `default` etc
    Fixed(Arc<T>),
}

impl<T> ConfigHandle<T>
where
    T: Send + Sync + 'static,
{
    /// Fetch the current version of the config referred to by this handle
    /// Return is an `Arc` so that if the config is updated after you get it,
    /// you will simply own an outdated pointer
    pub fn get(&self) -> Arc<T> {
        match &self.inner {
            ConfigHandleImpl::Registered(handle) => handle.get(),
            ConfigHandleImpl::Fixed(contents) => contents.clone(),
        }
    }

    /// Method that returns a config update watcher that observes changes
    /// that are applied to the underlying config. Requesting a config watcher
    /// for static config (e.g. sourced via static JSON file) results in an error.
    /// A single instance of ConfigHandle can produce multiple ConfigUpdateWatchers
    /// but once the ConfigHandle gets dropped, the ConfigUpdateWatchers will no
    /// longer receive updates.
    pub fn watcher(&self) -> Result<ConfigUpdateWatcher<T>> {
        match &self.inner {
            ConfigHandleImpl::Registered(handle) => {
                Ok(ConfigUpdateWatcher::new(handle.update_receiver()))
            }
            ConfigHandleImpl::Fixed(_) => {
                bail!("Config update watchers are not supported for static configs")
            }
        }
    }

    pub(crate) fn from_registered(registered: Arc<RegisteredConfigEntity<T>>) -> Self {
        Self {
            inner: ConfigHandleImpl::Registered(registered),
        }
    }
}

impl<T> ConfigHandle<T>
where
    T: Send + Sync + DeserializeOwned + 'static,
{
    /// Create a static config handle from a JSON blob. Useful for testing.
    pub fn from_json(data: &str) -> Result<Self> {
        Ok(Self {
            inner: ConfigHandleImpl::Fixed(Arc::new(from_str(data)?)),
        })
    }
}

impl<T> Default for ConfigHandle<T>
where
    T: Send + Sync + Default + 'static,
{
    fn default() -> Self {
        Self {
            inner: ConfigHandleImpl::Fixed(Arc::new(T::default())),
        }
    }
}

impl<T> From<T> for ConfigHandle<T>
where
    T: Send + Sync + 'static,
{
    fn from(other: T) -> Self {
        Self {
            inner: ConfigHandleImpl::Fixed(Arc::new(other)),
        }
    }
}
