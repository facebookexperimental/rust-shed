/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

/// JustKnobs implementation that uses a cached_config abstraction as a backing
/// storage.  It's primarily used in tests.
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

use anyhow::Result;
use anyhow::anyhow;
use arc_swap::ArcSwap;
use cached_config::ConfigHandle;
use just_knobs_struct::JustKnobs as JustKnobsStruct;
use serde::Deserialize;
use serde::Serialize;
use tokio::runtime::Handle;
use tracing_slog_compat::Logger;
use tracing_slog_compat::debug;
use tracing_slog_compat::error;
use tracing_slog_compat::warn;

use crate::JustKnobs;

static JUST_KNOBS: OnceLock<ArcSwap<JustKnobsInMemory>> = OnceLock::new();
static JUST_KNOBS_WORKER_STATE: OnceLock<JustKnobsWorkerState> = OnceLock::new();

#[derive(Serialize, Deserialize)]
pub struct JustKnobsInMemory(HashMap<String, KnobVal>);

impl From<Arc<JustKnobsStruct>> for JustKnobsInMemory {
    fn from(jk: Arc<JustKnobsStruct>) -> Self {
        Self(
            jk.bools
                .iter()
                .map(|(k, v)| (k.clone(), KnobVal::Bool(*v)))
                .chain(jk.ints.iter().map(|(k, v)| (k.clone(), KnobVal::Int(*v))))
                .collect(),
        )
    }
}

#[derive(Serialize, Deserialize, Copy, Clone)]
#[serde(untagged)]
enum KnobVal {
    Bool(bool),
    Int(i64),
}

pub fn in_use() -> bool {
    JUST_KNOBS_WORKER_STATE.get().is_some() || JUST_KNOBS.get().is_some()
}

pub fn just_knobs() -> &'static ArcSwap<JustKnobsInMemory> {
    JUST_KNOBS.get_or_init(|| ArcSwap::from(Arc::new(JustKnobsInMemory(HashMap::new()))))
}

pub struct CachedConfigJustKnobs;
impl JustKnobs for CachedConfigJustKnobs {
    fn eval(name: &str, _hash_val: Option<&str>, _switch_val: Option<&str>) -> Result<bool> {
        let value = *(just_knobs()
            .load()
            .0
            .get(name)
            .ok_or_else(|| anyhow!("Missing just knobs bool: {}", name))?);

        match value {
            KnobVal::Int(_v) => Err(anyhow!(
                "JustKnobs knob {} has type int while expected bool",
                name,
            )),
            KnobVal::Bool(b) => Ok(b),
        }
    }

    fn get(name: &str, _switch_val: Option<&str>) -> Result<i64> {
        let value = *(just_knobs()
            .load()
            .0
            .get(name)
            .ok_or_else(|| anyhow!("Missing just knobs int: {}", name))?);

        match value {
            KnobVal::Bool(_v) => Err(anyhow!(
                "JustKnobs knob {} has type bool while expected int",
                name,
            )),
            KnobVal::Int(b) => Ok(b),
        }
    }
}

fn log_just_knobs(just_knobs: &JustKnobsStruct) -> String {
    serde_json::to_string(just_knobs)
        .unwrap_or_else(|e| format!("failed to serialize JustKnobs: {}", e))
}

pub fn init_just_knobs_worker(
    logger: impl IntoLogger,
    config_handle: ConfigHandle<JustKnobsStruct>,
    runtime_handle: Handle,
) -> Result<()> {
    let logger = logger.into_logger();
    init_just_knobs(&logger, &config_handle)?;
    if JUST_KNOBS_WORKER_STATE
        .set(JustKnobsWorkerState {
            config_handle,
            logger,
        })
        .is_err()
    {
        panic!("Two or more JustKnobs update threads exist at the same time");
    }
    runtime_handle.spawn(wait_and_update());

    Ok(())
}

pub fn init_just_knobs(
    logger: &(impl IntoLogger + Clone),
    config_handle: &ConfigHandle<JustKnobsStruct>,
) -> Result<()> {
    let just_knobs = config_handle.get();
    let logger = logger.clone();
    debug!(
        logger.into_logger(),
        "Initializing JustKnobs: {}",
        log_just_knobs(&just_knobs)
    );
    update_just_knobs(just_knobs)
}

struct JustKnobsWorkerState {
    config_handle: ConfigHandle<JustKnobsStruct>,
    logger: Logger,
}

async fn wait_and_update() {
    let state = JUST_KNOBS_WORKER_STATE
        .get()
        .expect("JustKnob worker state uninitialised");
    let mut config_watcher = state
        .config_handle
        .watcher()
        .expect("JustKnob backed by static config source");
    loop {
        match config_watcher.wait_for_next().await {
            Ok(new_just_knobs) => wait_and_update_iteration(new_just_knobs, &state.logger),
            Err(e) => {
                error!(
                    state.logger,
                    "Error in fetching latest config for just knob : {}.\n Exiting JustKnob updater",
                    e
                );
                // Set the refresh failure count counter so that the oncall can be alerted
                // based on this metric
                return;
            }
        }
    }
}

fn wait_and_update_iteration(new_just_knobs: Arc<JustKnobsStruct>, logger: &Logger) {
    debug!(
        logger,
        "Updating JustKnobs to new: {}",
        log_just_knobs(&new_just_knobs),
    );
    if let Err(e) = update_just_knobs(new_just_knobs) {
        warn!(logger, "Failed to refresh just knobs: {}", e);
    }
}

fn update_just_knobs(new_just_knobs: Arc<JustKnobsStruct>) -> Result<()> {
    let just_knobs = just_knobs();
    just_knobs.swap(Arc::new(new_just_knobs.into()));
    Ok(())
}

#[doc(hidden)]
pub trait IntoLogger {
    fn into_logger(self) -> tracing_slog_compat::Logger;
}

impl IntoLogger for slog::Logger {
    fn into_logger(self) -> tracing_slog_compat::Logger {
        tracing_slog_compat::Logger::Slog(self)
    }
}

impl IntoLogger for tracing_slog_compat::Logger {
    fn into_logger(self) -> tracing_slog_compat::Logger {
        self
    }
}

#[cfg(test)]
mod test {
    use std::thread;
    use std::time::Duration;

    use cached_config::ConfigStore;
    use cached_config::ModificationTime;
    use cached_config::test_source::TestSource;
    use slog_glog_fmt::logger_that_can_work_in_tests;
    use tokio::runtime::Handle;

    use super::*;
    use crate as justknobs;
    const SLEEP_TIME_MS: u64 = 50;

    #[tokio::test(start_paused = true)]
    async fn test_jk_loading() -> Result<()> {
        let test_source = {
            let test_source = TestSource::new();
            test_source.insert_config(
                "justknobs.json",
                r#"{ "bools": {"my/config:knob1": true } }"#,
                ModificationTime::UnixTimestamp(1),
            );
            Arc::new(test_source)
        };
        let logger = logger_that_can_work_in_tests().unwrap();
        let store = ConfigStore::new(test_source.clone(), Duration::from_millis(2), None);
        init_just_knobs_worker(
            logger.clone(),
            store.get_config_handle("justknobs.json".to_owned())?,
            Handle::current(),
        )?;
        assert!(CachedConfigJustKnobs::eval("my/config:knob1", None, None).unwrap());

        test_source.insert_config(
            "justknobs.json",
            r#"{
                "bools": {"my/config:knob1": false },
                "ints": {"my/config:knob2": 10 }
             }"#,
            ModificationTime::UnixTimestamp(10),
        );
        test_source.insert_to_refresh("justknobs.json".to_owned());
        // Ensure the updater thread has run
        thread::yield_now();
        thread::sleep(Duration::from_millis(SLEEP_TIME_MS));
        // This doesn't really sleep because we're running this test with paused
        // mode so the time is auto-advanced if the runtime has nothing to do.
        tokio::time::sleep(Duration::from_millis(SLEEP_TIME_MS)).await;

        assert!(!CachedConfigJustKnobs::eval("my/config:knob1", None, None).unwrap());
        assert_eq!(
            CachedConfigJustKnobs::get("my/config:knob2", None).unwrap(),
            10
        );
        assert!(CachedConfigJustKnobs::eval("my/config:knob3", None, None).is_err());
        Ok(())
    }

    #[cfg(fbcode_build)]
    #[fbinit::test]
    fn test_jk_loading_with_static_config_handle() -> Result<()> {
        let config_handle = ConfigHandle::from_json(r#"{ "bools": {"my/config:knob1": true } }"#)?;
        let logger = logger_that_can_work_in_tests().unwrap();
        init_just_knobs(&logger, &config_handle)?;
        assert!(CachedConfigJustKnobs::eval("my/config:knob1", None, None).unwrap());
        assert!(CachedConfigJustKnobs::eval("my/config:knob2", None, None).is_err());

        assert!(justknobs::eval("my/config:knob1", None, None).unwrap());
        assert!(justknobs::eval("my/config:knob2", None, None).is_err());
        Ok(())
    }
}
