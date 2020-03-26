/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use fbinit::FacebookInit;
use std::time::Duration;

use stats_traits::{
    stat_types::{
        BoxCounter, BoxHistogram, BoxTimeseries, Counter, Histogram, SingletonCounter, Timeseries,
    },
    stats_manager::{
        AggregationType, BoxStatsManager, BucketConfig, StatsManager, StatsManagerFactory,
    },
};

pub struct NoopStatsFactory;

impl StatsManagerFactory for NoopStatsFactory {
    fn create(&self) -> BoxStatsManager {
        Box::new(Noop)
    }
}

pub struct Noop;

impl StatsManager for Noop {
    fn aggregate(&self) {}

    fn create_counter(&self, _name: &str) -> BoxCounter {
        Box::new(Noop)
    }

    fn create_timeseries(
        &self,
        _name: &str,
        _aggregation_types: &[AggregationType],
        _intervals: &[Duration],
    ) -> BoxTimeseries {
        Box::new(Noop)
    }

    fn create_histogram(
        &self,
        _name: &str,
        _aggregation_types: &[AggregationType],
        _conf: BucketConfig,
        _percentiles: &[u8],
    ) -> BoxHistogram {
        Box::new(Noop)
    }
}

impl Counter for Noop {
    fn increment_value(&self, _value: i64) {}
}

impl Timeseries for Noop {
    fn add_value(&self, _value: i64) {}
    fn add_value_aggregated(&self, _value: i64, _nsamples: u32) {}
}

impl Histogram for Noop {
    fn add_value(&self, _value: i64) {}
    fn add_repeated_value(&self, _value: i64, _nsamples: u32) {}
}

impl SingletonCounter for Noop {
    fn set_value(&self, _fb: FacebookInit, _value: i64) {}
    fn get_value(&self, _fb: FacebookInit) -> Option<i64> {
        None
    }
}
