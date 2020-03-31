/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde_json::from_str;
use slog::{info, warn, Logger};
use std::{
    collections::HashMap,
    fmt,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, Weak},
    thread,
    time::Duration,
};

use crate::file_source::FileSource;
use crate::handle::ConfigHandle;
use crate::refreshable_entities::{Refreshable, RegisteredConfigEntity};
use crate::Source;

/// A wrapper around the configerator APIs to provide an easily mocked way of reading JSON configs
/// into Serde-compatible structures.
#[derive(Clone)]
pub struct ConfigStore {
    source: Arc<dyn Source + Sync + Send>,
    clients: Arc<Mutex<HashMap<String, ClientList>>>,
    kick: Arc<Condvar>,
    logger: Option<Logger>,
}

type ClientList = Vec<Weak<dyn Refreshable + Sync + Send>>;

impl ConfigStore {
    /// Create a new instance of the ConfigStore with its own updating thread
    /// which will be run every `poll_interval`. The configs will be retrieved
    /// from the provided `source`. If `logger` is given then the store will
    /// inform about status of refreshes.
    ///
    /// TODO: Each instance creates its own thread, make sure the thread is
    /// stopped once the ConfigStore and all relevant ConfigHandle are destroyed.
    pub fn new(
        source: Arc<dyn Source + Sync + Send>,
        poll_interval: Duration,
        logger: Option<Logger>,
    ) -> Self {
        let this = Self {
            source,
            clients: Arc::new(Mutex::new(HashMap::new())),
            kick: Arc::new(Condvar::new()),
            logger,
        };

        thread::Builder::new()
            .name("rust-cfgr-updates".into())
            .spawn({
                let this = this.clone();
                move || this.updater_thread(poll_interval)
            })
            .expect("Can't spawn cached_config updates poller");

        this
    }

    /// Get configs from JSON files on disk.
    /// `logger` is `None` if no desire to log from the background thread, or a `slog::Logger` to log to.
    /// `prefix` is the directory prefix to apply to all config paths to find the on-disk JSON
    /// `suffix` is a file suffix to add to get the config JSON
    /// `poll_interval` is the sleep time between checks for config changes
    pub fn file(
        logger: impl Into<Option<Logger>>,
        directory: PathBuf,
        extension: impl Into<Option<String>>,
        poll_interval: Duration,
    ) -> Self {
        Self::new(
            Arc::new(FileSource::new(directory, extension)),
            poll_interval,
            logger.into(),
        )
    }

    /// Fetch a self-updating config handle for the config at `path`.
    /// See `ConfigHandle` for uses of this handle.
    pub fn get_config_handle<T>(&self, path: String) -> Result<ConfigHandle<T>>
    where
        T: Send + Sync + DeserializeOwned + 'static,
    {
        let entity = {
            let entity = self.source.config_for_path(&path)?;
            Arc::new(RegisteredConfigEntity::new(
                path.clone(),
                entity.mod_time,
                entity.version,
                Arc::new(from_str(&entity.contents)?),
            ))
        };

        let mut clients = self.clients.lock().expect("lock poisoned");

        let client_handle = clients.entry(path).or_insert_with(Vec::new);
        client_handle.push(Arc::downgrade(&entity) as Weak<dyn Refreshable + Send + Sync>);

        self.kick.notify_one();

        Ok(ConfigHandle::from_registered(entity))
    }

    fn refresh_client(&self, client: Arc<dyn Refreshable + Sync + Send>) {
        let res = self
            .source
            .config_for_path(client.get_path())
            .and_then(|entity| client.refresh(entity));
        if let Some(ref logger) = self.logger {
            match res {
                Ok(false) => {}
                Ok(true) => info!(logger, "Updated path {}", client.get_path()),
                Err(e) => warn!(
                    logger,
                    "Failed to update path {} due to {:#?}",
                    client.get_path(),
                    e
                ),
            }
        }
    }

    fn refresh_client_list(&self, client_list: &[Weak<dyn Refreshable + Sync + Send>]) {
        for client in client_list {
            if let Some(client) = client.upgrade() {
                self.refresh_client(client);
            }
        }
    }

    fn updater_thread(&self, poll_interval: Duration) {
        loop {
            let mut clients = self.clients.lock().expect("lock poisoned");

            for path in self
                .source
                .paths_to_refresh(&mut clients.keys().map(|x| -> &str { x }))
            {
                if let Some(client_list) = clients.get(path) {
                    self.refresh_client_list(client_list);
                }
            }

            // Remove lost clients
            clients.retain(|_, client_list| {
                // Remove all vanished clients from the list, then check if it's empty
                client_list.retain(|client| client.upgrade().is_some());
                !client_list.is_empty()
            });

            if clients.is_empty() {
                // Don't loop when there are no active clients to care about
                let _ = self.kick.wait(clients);
            }

            thread::sleep(poll_interval);
        }
    }
}

impl fmt::Debug for ConfigStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConfigStore({:?})", self.source)
    }
}
