/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Provides dynamic versions of the thread local stats. Dynamic here means that the name of the
//! counter is being decided in runtime. If you use the `define_stats!` to define a dynamic stat
//! then the pattern that is used to format the key and the arguments used in that pattern are
//! statically checked.

use std::cell::RefCell;
use std::collections::HashMap;
use std::thread::LocalKey;

use fbinit::FacebookInit;

use crate::stat_types::BoxCounter;
use crate::stat_types::BoxHistogram;
use crate::stat_types::BoxSingletonCounter;
use crate::stat_types::BoxTimeseries;
use crate::stat_types::Counter;
use crate::stat_types::Histogram;
use crate::stat_types::SingletonCounter;
use crate::stat_types::Timeseries;

/// The struct to hold key and stat generators that are later being used in runtime to create new
/// stats that are being held in a map to avoid reconstruction of the same counter.
pub struct DynamicStat<T, TStatType> {
    map: RefCell<HashMap<String, TStatType>>,
    key_generator: fn(&T) -> String,
    stat_generator: fn(&str) -> TStatType,
}

impl<T, TStatType> DynamicStat<T, TStatType> {
    pub fn new(key_generator: fn(&T) -> String, stat_generator: fn(&str) -> TStatType) -> Self {
        DynamicStat {
            map: RefCell::new(HashMap::new()),
            key_generator,
            stat_generator,
        }
    }

    fn get_or_default<F, V>(&self, args: T, cb: F) -> V
    where
        F: FnOnce(&TStatType) -> V,
    {
        // The HashMap::entry requires to pass the key by value, so we can't create the string
        // once and pass it to entry and later to the closure. That is why we are creating the
        // key twice if there is no entry for it already.
        let mut map = self.map.borrow_mut();
        let entry = map.entry((self.key_generator)(&args));
        cb(entry.or_insert_with(|| (self.stat_generator)(&(self.key_generator)(&args))))
    }
}

/// Similar to the Counter trait, but accepts the args parameter for accessing dynamic counters
/// created at runtime.
pub trait DynamicCounter<'a, T> {
    /// Dynamic version of `Counter::increment_value`
    fn increment_value(&'a self, value: i64, args: T);
}

impl<'a, T> DynamicCounter<'a, T> for DynamicStat<T, BoxCounter> {
    fn increment_value(&'a self, value: i64, args: T) {
        self.get_or_default(args, |s| s.increment_value(value));
    }
}

impl<T> DynamicCounter<'static, T> for LocalKey<DynamicStat<T, BoxCounter>> {
    fn increment_value(&'static self, value: i64, args: T) {
        self.with(|s| s.increment_value(value, args));
    }
}

/// Similar to Timeseries trait, but accepts the args parameter for accessing dynamic timeseries
/// created in runtime.
pub trait DynamicTimeseries<'a, T> {
    /// Dynamic version of `Timeseries::add_value`
    fn add_value(&'a self, value: i64, args: T);

    /// Dynamic version of `Timeseries::add_value_aggregated`
    fn add_value_aggregated(&'a self, value: i64, nsamples: u32, args: T);
}

impl<'a, T> DynamicTimeseries<'a, T> for DynamicStat<T, BoxTimeseries> {
    fn add_value(&'a self, value: i64, args: T) {
        self.get_or_default(args, |s| s.add_value(value));
    }

    fn add_value_aggregated(&'a self, value: i64, nsamples: u32, args: T) {
        self.get_or_default(args, |s| s.add_value_aggregated(value, nsamples));
    }
}

impl<T> DynamicTimeseries<'static, T> for LocalKey<DynamicStat<T, BoxTimeseries>> {
    fn add_value(&'static self, value: i64, args: T) {
        self.with(|s| s.add_value(value, args));
    }

    fn add_value_aggregated(&'static self, value: i64, nsamples: u32, args: T) {
        self.with(|s| s.add_value_aggregated(value, nsamples, args));
    }
}

/// Similar to the Histogram trait, but accepts the args parameter for accessing dynamic
/// histograms created at runtime.
pub trait DynamicHistogram<'a, T> {
    /// Dynamic version of `Histogram::add_value`
    fn add_value(&'a self, value: i64, args: T);

    /// Dynamic version of `Histogram::add_repeated_value`
    fn add_repeated_value(&'a self, value: i64, nsamples: u32, args: T);
}

impl<'a, T> DynamicHistogram<'a, T> for DynamicStat<T, BoxHistogram> {
    fn add_value(&'a self, value: i64, args: T) {
        self.get_or_default(args, |s| s.add_value(value));
    }

    fn add_repeated_value(&'a self, value: i64, nsamples: u32, args: T) {
        self.get_or_default(args, |s| s.add_repeated_value(value, nsamples));
    }
}

impl<T> DynamicHistogram<'static, T> for LocalKey<DynamicStat<T, BoxHistogram>> {
    fn add_value(&'static self, value: i64, args: T) {
        self.with(|s| s.add_value(value, args));
    }

    fn add_repeated_value(&'static self, value: i64, nsamples: u32, args: T) {
        self.with(|s| s.add_repeated_value(value, nsamples, args));
    }
}

/// Similar to the SingletonCounter trait, but accepts the args parameter for accessing dynamic
/// histograms created at runtime.
pub trait DynamicSingletonCounter<'a, T> {
    /// Dynamic version of `SingletonCounter::set_value`
    fn set_value(&'a self, fb: FacebookInit, value: i64, args: T);

    /// Dynamic version of `SingletonCounter::get_value`
    fn get_value(&'a self, fb: FacebookInit, args: T) -> Option<i64>;

    /// Dynamic version of `SingletonCounter::increment_value`
    fn increment_value(&'a self, fb: FacebookInit, value: i64, args: T);
}

impl<'a, T> DynamicSingletonCounter<'a, T> for DynamicStat<T, BoxSingletonCounter> {
    fn set_value(&'a self, fb: FacebookInit, value: i64, args: T) {
        self.get_or_default(args, |s| s.set_value(fb, value));
    }

    fn get_value(&'a self, fb: FacebookInit, args: T) -> Option<i64> {
        self.get_or_default(args, |s| s.get_value(fb))
    }

    fn increment_value(&'a self, fb: FacebookInit, value: i64, args: T) {
        self.get_or_default(args, |s| s.increment_value(fb, value))
    }
}

impl<T> DynamicSingletonCounter<'static, T> for LocalKey<DynamicStat<T, BoxSingletonCounter>> {
    fn set_value(&'static self, fb: FacebookInit, value: i64, args: T) {
        self.with(|s| s.set_value(fb, value, args))
    }

    fn get_value(&'static self, fb: FacebookInit, args: T) -> Option<i64> {
        self.with(|s| s.get_value(fb, args))
    }

    fn increment_value(&'static self, fb: FacebookInit, value: i64, args: T) {
        self.with(|s| s.increment_value(fb, value, args))
    }
}
