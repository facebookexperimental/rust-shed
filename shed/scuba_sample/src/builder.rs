/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! See the [ScubaSampleBuilder] documentation

use fbinit::FacebookInit;
use serde_json::{Error, Value};
use std::collections::hash_map::Entry;
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{Error as IoError, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::sample::ScubaSample;
use crate::value::ScubaValue;

/// A helper builder to make it easier to create a new sample and log it into
/// the proper Scuba dataset.
#[derive(Clone)]
pub struct ScubaSampleBuilder {
    sample: ScubaSample,
    log_file: Option<Arc<Mutex<File>>>,
}

impl ScubaSampleBuilder {
    /// Create a new instance of the Builder with initially an empty sample
    /// that will preserve the sample in the provided dataset. The arguments
    /// are used only in fbcode builds.
    pub fn new<T: Into<String>>(_fb: FacebookInit, _dataset: T) -> Self {
        Self::with_discard()
    }

    /// Create a new instance of the Builder with initially an empty sample
    /// that will discard the sample instead of writing it to a Scuba dataset.
    pub fn with_discard() -> Self {
        Self {
            sample: ScubaSample::new(),
            log_file: None,
        }
    }

    /// Create a new instance of the Builder with initially an empty sample
    /// that will preserve the sample in the provided log file.
    pub fn with_log_file<L: AsRef<Path>>(mut self, log_file: L) -> Result<Self, IoError> {
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;
        self.log_file = Some(Arc::new(Mutex::new(log_file)));
        Ok(self)
    }

    /// Return true if a client is not set for this builder. This method will
    /// return false even if a log file is provided and the sample will be
    /// preserved in it.
    pub fn is_discard(&self) -> bool {
        true
    }

    /// Call the internal sample's [super::sample::ScubaSample::add] method
    pub fn add<K: Into<String>, V: Into<ScubaValue>>(&mut self, key: K, value: V) -> &mut Self {
        self.sample.add(key, value);
        self
    }

    /// Call the internal sample's [super::sample::ScubaSample::remove] method
    pub fn remove<K: Into<String>>(&mut self, key: K) -> &mut Self {
        self.sample.remove(key);
        self
    }

    /// Call the internal sample's [super::sample::ScubaSample::get] method
    pub fn get<K: Into<String>>(&self, key: K) -> Option<&ScubaValue> {
        self.sample.get(key)
    }

    /// Call the internal sample's [super::sample::ScubaSample::entry] method
    pub fn entry<K: Into<String>>(&mut self, key: K) -> Entry<String, ScubaValue> {
        self.sample.entry(key)
    }

    /// Get a reference to the internally built sample.
    pub fn get_sample(&self) -> &ScubaSample {
        &self.sample
    }

    /// Log the internally built sample the previously configured log file with
    /// overriding it's timestampt to current time.
    pub fn log(&mut self) {
        self.sample.set_time_now();
        if let Some(ref log_file) = self.log_file {
            if let Ok(sample) = self.to_json() {
                let mut log_file = log_file.lock().expect("Poisoned lock");
                let _ = log_file.write_all(sample.to_string().as_bytes());
                let _ = log_file.write_all(b"\n");
            }
        }
    }

    /// Log the internally built sample to the previously configured log file
    /// with overriding it's timestampt to provided time.
    pub fn log_with_time(&mut self, time: u64) {
        self.sample.set_time(time);
        if let Some(ref log_file) = self.log_file {
            if let Ok(sample) = self.sample.to_json() {
                let mut log_file = log_file.lock().expect("Poisoned lock");
                let _ = log_file.write_all(sample.to_string().as_bytes());
                let _ = log_file.write_all(b"\n");
            }
        }
    }

    /// Either flush the configured client with the provided timeout or flush
    /// the configured log file making sure all the logged samples have been
    /// written to it. The timeout is used only in fbcode builds.
    pub fn flush(&self, _timeout: i64) {
        if let Some(ref log_file) = self.log_file {
            let mut log_file = log_file.lock().expect("Poisoned lock");
            let _ = log_file.flush();
        }
    }

    /// Return a json serialized sample
    pub fn to_json(&self) -> Result<Value, Error> {
        self.sample.to_json()
    }

    /// Add values to the sample that are widely used in Facebook services. For
    /// non-fbcode-builds it does nothing. The provided mapper function is used
    /// to transform the valuse before they are written to the sample.
    pub fn add_mapped_common_server_data<F>(&mut self, _mapper: F) -> &mut Self
    where
        F: Fn(ServerData) -> &'static str,
    {
        self
    }

    /// Add values to the sample that are widely used in Facebook services. For
    /// non-fbcode-builds it does nothing.
    pub fn add_common_server_data(&mut self) -> &mut Self {
        self.add_mapped_common_server_data(|data| data.default_key())
    }
}

impl fmt::Debug for ScubaSampleBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ScubaSampleBuilder {{ sample: {:?} }}", self.sample)
    }
}

/// Enum representing commonly used server data written to the Scuba sample.
pub enum ServerData {
    /// Hostname of the server
    Hostname,
    /// Tier of the service
    Tier,
    /// Tupperware TaskId of the service
    TaskId,
    /// Tupperware CanaryId of the service
    CanaryId,
    /// Tupperware JobHandle of the service
    JobHandle,
    /// Build revision of the current binary
    BuildRevision,
    /// Build rule of the current binary
    BuildRule,
}

impl ServerData {
    /// Return a unique key for the server data under which the value will be
    /// stored in the sample. Pay attention not to use the same keys if you don't
    /// wish to override those values.
    pub fn default_key(&self) -> &'static str {
        match self {
            ServerData::Hostname => "server_hostname",
            ServerData::Tier => "server_tier",
            ServerData::TaskId => "tw_task_id",
            ServerData::CanaryId => "tw_canary_id",
            ServerData::JobHandle => "tw_handle",
            ServerData::BuildRevision => "build_revision",
            ServerData::BuildRule => "build_rule",
        }
    }
}
