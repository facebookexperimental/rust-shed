/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Provides a macro `define_stats!` for creation of stats. This crate requires the caller to
//! schedule aggregation of stats by calling schedule_stats_aggregation and executing the returned
//! future.

#![deny(warnings, missing_docs, clippy::all, broken_intra_doc_links)]

pub mod macros;
mod noop_stats;
pub mod thread_local_aggregator;

pub mod prelude {
    //! A "prelude" of `stats` crate.
    //!
    //! This prelude is similar to the standard library's prelude in that you'll
    //! almost always want to import its entire contents, but unlike the standard
    //! library's prelude you'll have to do so manually:
    //!
    //! ```
    //! # #![allow(unused)]
    //! use stats::prelude::*;
    //! ```
    pub use crate::{define_stats, define_stats_struct};
    pub use stats_traits::{
        dynamic_stat_types::{
            DynamicCounter, DynamicHistogram, DynamicSingletonCounter, DynamicTimeseries,
        },
        stat_types::{
            Counter, CounterStatic, Histogram, HistogramStatic, Timeseries, TimeseriesStatic,
        },
    };
}

use std::sync::RwLock;

use lazy_static::lazy_static;
use stats_traits::{
    stat_types::BoxSingletonCounter,
    stats_manager::{BoxStatsManager, StatsManagerFactory},
};

pub use self::thread_local_aggregator::schedule_stats_aggregation_preview;

lazy_static! {
    static ref STATS_MANAGER_FACTORY: RwLock<Option<Box<dyn StatsManagerFactory + Send + Sync>>> =
        RwLock::new(None);
}

/// This function must be called exactly once before accessing any of the stats,
/// otherwise it will panic.
/// If it won't be called a default stats manager factory will be assumed that
/// does nothing. (Facebook only: the default will use fb303 counters)
pub fn register_stats_manager_factory(factory: impl StatsManagerFactory + Send + Sync + 'static) {
    let mut global_factory = STATS_MANAGER_FACTORY.write().expect("poisoned lock");
    assert!(
        global_factory.is_none(),
        "Called stats::stats_manager::register_stats_manager_factory more than once"
    );
    global_factory.replace(Box::new(factory));
}

#[doc(hidden)]
/// You probably don't have to use this function, it is made public so that it
/// might be used by the macros in this crate. It reads the globally registered
/// StatsManagerFactory and creates a new instance of StatsManager.
pub fn create_stats_manager() -> BoxStatsManager {
    if let Some(factory) = STATS_MANAGER_FACTORY
        .read()
        .expect("poisoned lock")
        .as_ref()
    {
        return factory.create();
    }
    // We get here only if register_stats_manager_factory was not called yet
    // but we have to keep in mind this is a race so first get hold of write
    // lock and check if the factory is still unset.
    let mut write_lock = STATS_MANAGER_FACTORY.write().expect("poisoned lock");
    let factory = write_lock.get_or_insert_with(get_default_stats_manager_factory);
    factory.create()
}

fn get_default_stats_manager_factory() -> Box<dyn StatsManagerFactory + Send + Sync> {
    #[cfg(fbcode_build)]
    {
        Box::new(::stats_facebook::ThreadLocalStatsFactory)
    }
    #[cfg(not(fbcode_build))]
    {
        Box::new(crate::noop_stats::NoopStatsFactory)
    }
}

#[doc(hidden)]
/// You probably don't have to use this function, it is made public so that it
/// might be used by the macros in this crate. It creates a new SingletonCounter.
pub fn create_singleton_counter(name: String) -> BoxSingletonCounter {
    #[cfg(fbcode_build)]
    {
        Box::new(::stats_facebook::singleton_counter::ServiceDataSingletonCounter::new(name))
    }

    #[cfg(not(fbcode_build))]
    {
        let _ = name;
        Box::new(crate::noop_stats::Noop)
    }
}
