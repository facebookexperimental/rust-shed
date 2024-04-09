// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use std::time::Duration;

use anyhow::Result;
use configerator_client::cpp_sync_client::ConfigeratorCppClient;
use fbinit::FacebookInit;
use slog::Logger;

use crate::ConfigStore;
use crate::Entity;
use crate::ModificationTime;
use crate::Source;

impl ConfigStore {
    /// Get configs from the Configerator service.
    /// `logger` is `None` if no desire to log from the background thread, or a `slog::Logger` to log to.
    /// `poll_interval` is the sleep time between checks for config changes
    /// `refresh_timeout` is the maximum time permitted to fetch one config from Configerator
    /// in the background thread
    pub fn configerator(
        fb: FacebookInit,
        logger: impl Into<Option<Logger>>,
        poll_interval: impl Into<Option<Duration>>,
        refresh_timeout: Duration,
    ) -> Result<Self> {
        let api = ConfigeratorCppClient::new(fb)?;
        Self::configerator_impl(api, logger.into(), poll_interval.into(), refresh_timeout)
    }

    /// Support for [signed configs]
    ///
    /// Same parameters as `configerator` above, but also takes a map of
    /// config names (configerator paths) to crypto projects
    ///
    /// [signed configs]: https://fburl.com/wiki/ovnmsktg
    pub fn signed_configerator(
        fb: FacebookInit,
        logger: impl Into<Option<Logger>>,
        config_name_to_crypto_project: HashMap<String, String>,
        poll_interval: impl Into<Option<Duration>>,
        refresh_timeout: Duration,
    ) -> Result<Self> {
        let api = ConfigeratorCppClient::with_signed_config_validation(
            fb,
            config_name_to_crypto_project,
        )?;
        Self::configerator_impl(api, logger.into(), poll_interval.into(), refresh_timeout)
    }

    /// Support for [signed configs]
    ///
    /// Same parameters as `configerator` above, but also takes a list of
    /// regexes mapping configs to crypto projects
    ///
    /// [signed configs]: https://fburl.com/wiki/ovnmsktg
    pub fn regex_signed_configerator(
        fb: FacebookInit,
        logger: impl Into<Option<Logger>>,
        regex_crypto_projects: Vec<(String, String)>,
        poll_interval: impl Into<Option<Duration>>,
        refresh_timeout: Duration,
    ) -> Result<Self> {
        let api =
            ConfigeratorCppClient::with_regex_signed_config_validation(fb, regex_crypto_projects)?;
        Self::configerator_impl(api, logger.into(), poll_interval.into(), refresh_timeout)
    }

    fn configerator_impl(
        mut api: ConfigeratorCppClient,
        logger: Option<Logger>,
        poll_interval: Option<Duration>,
        refresh_timeout: Duration,
    ) -> Result<Self> {
        let updates = Arc::new(Mutex::new(HashSet::new()));
        api.subscribe_for_updates({
            let updates = Arc::clone(&updates);
            move |configs| {
                updates
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .extend(configs)
            }
        })?;
        Ok(Self::new(
            Arc::new(ConfigeratorSource {
                refresh_timeout,
                api: Arc::new(api),
                updates,
            }),
            poll_interval,
            logger,
        ))
    }

    /// A utility form of `file` above - given a path to a configerator/materialized_configs, will read from it
    /// directly. Useful for SEV situations
    pub fn materialized_configs(
        logger: impl Into<Option<Logger>>,
        path: PathBuf,
        poll_interval: impl Into<Option<Duration>>,
    ) -> Self {
        Self::file(
            logger,
            path,
            String::from(".materialized_JSON"),
            poll_interval,
        )
    }
}

struct ConfigeratorSource {
    refresh_timeout: Duration,
    api: Arc<ConfigeratorCppClient>,
    updates: Arc<Mutex<HashSet<String>>>,
}

impl Source for ConfigeratorSource {
    fn config_for_path(&self, path: &str) -> Result<Entity> {
        let configerator_entity = self.api.get_entity(path, Some(self.refresh_timeout))?;
        Ok(Entity {
            contents: configerator_entity.contents,
            mod_time: match configerator_entity.mod_time {
                configerator_client::ModificationTime::Unset => ModificationTime::Unset,
                configerator_client::ModificationTime::UnixTimestamp(t) => {
                    ModificationTime::UnixTimestamp(t)
                }
                configerator_client::ModificationTime::DateTime(t) => ModificationTime::DateTime(t),
            },
            version: configerator_entity.version,
        })
    }

    fn paths_to_refresh<'a>(&self, paths: &mut dyn Iterator<Item = &'a str>) -> Vec<&'a str> {
        let configerator_paths =
            mem::take(&mut *self.updates.lock().unwrap_or_else(PoisonError::into_inner));
        paths.filter(|p| configerator_paths.contains(*p)).collect()
    }
}

impl fmt::Debug for ConfigeratorSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ConfigeratorSource")
    }
}
