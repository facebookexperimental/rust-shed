/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

// This module defines only macros which don't show up on module level
// documentation anyway so hide it.
#![doc(hidden)]

#[doc(hidden)]
pub mod common_macro_prelude {
    pub use lazy_static::lazy_static;
    pub use perthread::PerThread;
    pub use perthread::ThreadMap;
    pub use stats_traits::dynamic_stat_types::DynamicStat;
    pub use stats_traits::stat_types::BoxCounter;
    pub use stats_traits::stat_types::BoxHistogram;
    pub use stats_traits::stat_types::BoxSingletonCounter;
    pub use stats_traits::stat_types::BoxTimeseries;
    pub use stats_traits::stats_manager::AggregationType::*;
    pub use stats_traits::stats_manager::BoxStatsManager;
    pub use stats_traits::stats_manager::BucketConfig;
    pub use stats_traits::stats_manager::StatsManager;
    pub use std::sync::Arc;
    pub use std::time::Duration;

    pub use crate::create_singleton_counter;
    pub use crate::create_stats_manager;
    pub use crate::thread_local_aggregator::create_map;
}

/// The macro to define STATS module that contains static variables, one per counter you want to
/// export. This is the main and recomended way to interact with statistics provided by this crate.
/// If non empty prefix is passed then the exported counter name will be "{prefix}.{name}"
///
/// Examples:
/// ```
/// use stats::prelude::*;
/// use fbinit::FacebookInit;
///
/// define_stats! {
///     prefix = "my.test.counters";
///     manual_c: singleton_counter(),
///     test_c: counter(),
///     test_c2: counter("test_c.two"),
///     test_t: timeseries(Sum, Average),
///     test_t2: timeseries("test_t.two"; Sum, Average),
///     test_h: histogram(1, 0, 1000, Sum; P 99; P 50),
///     dtest_c: dynamic_counter("test_c.{}", (job: u64)),
///     dtest_t: dynamic_timeseries("test_t.{}", (region: &'static str); Rate, Sum),
///     dtest_t2: dynamic_timeseries("test_t.two.{}.{}", (job: u64, region: &'static str); Count),
///     dtest_h: dynamic_histogram("test_h.{}", (region: &'static str); 1, 0, 1000, Sum; P 99),
/// }
///
/// #[allow(non_snake_case)]
/// mod ALT_STATS {
///     use stats::define_stats;
///     define_stats! {
///         test_t: timeseries(Sum, Average),
///         test_t2: timeseries("test.two"; Sum, Average),
///     }
///     pub use self::STATS::*;
/// }
///
/// # #[allow(clippy::needless_doctest_main)]
/// #[fbinit::main]
/// fn main(fb: FacebookInit) {
///     STATS::manual_c.set_value(fb, 1);
///     STATS::test_c.increment_value(1);
///     STATS::test_c2.increment_value(100);
///     STATS::test_t.add_value(1);
///     STATS::test_t2.add_value_aggregated(79, 10);  // Add 79 and note it came from 10 samples
///     STATS::test_h.add_value(1);
///     STATS::test_h.add_repeated_value(1, 44);  // 44 times repeat adding 1
///     STATS::dtest_c.increment_value(7, (1000,));
///     STATS::dtest_t.add_value(77, ("lla",));
///     STATS::dtest_t2.add_value_aggregated(81, 12, (7, "lla"));
///     STATS::dtest_h.add_value(2, ("frc",));
///
///     ALT_STATS::test_t.add_value(1);
///     ALT_STATS::test_t2.add_value(1);
/// }
/// ```
#[macro_export]
macro_rules! define_stats {
    // Fill the optional prefix with empty string, all matching is repeated here to avoid the
    // recursion limit reached error in case the macro is misused.
    ($( $name:ident: $stat_type:tt($( $params:tt )*), )*) =>
        (define_stats!(prefix = ""; $( $name: $stat_type($( $params )*), )*););

    (prefix = $prefix:expr;
     $( $name:ident: $stat_type:tt($( $params:tt )*), )*) => (
        #[allow(non_snake_case, non_upper_case_globals, unused_imports)]
        pub(crate) mod STATS {
            use $crate::macros::common_macro_prelude::*;

            lazy_static! {
                static ref STATS_MAP: Arc<ThreadMap<BoxStatsManager>> = create_map();
            }

            thread_local! {
                static TL_STATS: PerThread<BoxStatsManager> =
                    STATS_MAP.register(create_stats_manager());
            }

            $( $crate::__define_stat!($prefix; $name: $stat_type($( $params )*)); )*
        }
    );
}

#[doc(hidden)]
#[macro_export]
macro_rules! __define_key_generator {
    ($name:ident($prefix:expr, $key:expr; $( $placeholder:ident: $type:ty ),+)) => (
        fn $name(&($( ref $placeholder, )+): &($( $type, )+)) -> String {
            let key = format!($key, $( $placeholder ),+);
            if $prefix.is_empty() {
                key
            } else {
                [$prefix, &key].join(".")
            }
        }
    );
}

#[doc(hidden)]
#[macro_export]
macro_rules! __define_stat {
    ($prefix:expr; $name:ident: singleton_counter()) => (
        $crate::__define_stat!($prefix; $name: singleton_counter(stringify!($name)));
    );

    ($prefix:expr; $name:ident: singleton_counter($key:expr)) => (
        lazy_static! {
            pub static ref $name: BoxSingletonCounter = create_singleton_counter($crate::__create_stat_key!($prefix, $key).to_string());
        }
    );

    ($prefix:expr; $name:ident: counter()) => (
        $crate::__define_stat!($prefix; $name: counter(stringify!($name)));
    );

    ($prefix:expr; $name:ident: counter($key:expr)) => (
        thread_local! {
            pub static $name: BoxCounter = TL_STATS.with(|stats| {
                stats.create_counter(&$crate::__create_stat_key!($prefix, $key))
            });
        }
    );

    // There are 4 inputs we use to produce a timeseries: the the prefix, the name (used in
    // STATS::name), the key (used in ODS or to query the key), the export types (SUM, RATE, etc.),
    // and the intervals (e.g. 60, 600). The key defaults to the name, and the intervals default to
    // whatever default Folly uses (which happens to be 60, 600, 3600);
    ($prefix:expr; $name:ident: timeseries($( $aggregation_type:expr ),*)) => (
        $crate::__define_stat!($prefix; $name: timeseries(stringify!($name); $( $aggregation_type ),*));
    );
    ($prefix:expr; $name:ident: timeseries($key:expr; $( $aggregation_type:expr ),*)) => (
        $crate::__define_stat!($prefix; $name: timeseries($key; $( $aggregation_type ),* ; ));
    );
    ($prefix:expr; $name:ident: timeseries($key:expr; $( $aggregation_type:expr ),* ; $( $interval: expr ),*)) => (
        thread_local! {
            pub static $name: BoxTimeseries = TL_STATS.with(|stats| {
                stats.create_timeseries(
                    &$crate::__create_stat_key!($prefix, $key),
                    &[$( $aggregation_type ),*],
                    &[$( $interval ),*]
                )
            });
        }
    );

    ($prefix:expr;
     $name:ident: histogram($bucket_width:expr,
                            $min:expr,
                            $max:expr
                            $(, $aggregation_type:expr )*
                            $(; P $percentile:expr )*)) => (
        $crate::__define_stat!($prefix;
                      $name: histogram(stringify!($name);
                                       $bucket_width,
                                       $min,
                                       $max
                                       $(, $aggregation_type )*
                                       $(; P $percentile )*));
    );

    ($prefix:expr;
     $name:ident: histogram($key:expr;
                            $bucket_width:expr,
                            $min:expr,
                            $max:expr
                            $(, $aggregation_type:expr )*
                            $(; P $percentile:expr )*)) => (
        thread_local! {
            pub static $name: BoxHistogram = TL_STATS.with(|stats| {
                stats.create_histogram(
                    &$crate::__create_stat_key!($prefix, $key),
                    &[$( $aggregation_type ),*],
                    BucketConfig {
                        width: $bucket_width,
                        min: $min,
                        max: $max,
                    },
                    &[$( $percentile ),*])
            });
        }
    );

    ($prefix:expr;
     $name:ident: dynamic_singleton_counter($key:expr, ($( $placeholder:ident: $type:ty ),+))) => (
        thread_local! {
            pub static $name: DynamicStat<($( $type, )+), BoxSingletonCounter> = {
                $crate::__define_key_generator!(
                    __key_generator($prefix, $key; $( $placeholder: $type ),+)
                );

                fn __stat_generator(key: &str) -> BoxSingletonCounter {
                    create_singleton_counter(key.to_string())
                }

                DynamicStat::new(__key_generator, __stat_generator)
            }
        }
    );

    ($prefix:expr;
     $name:ident: dynamic_counter($key:expr, ($( $placeholder:ident: $type:ty ),+))) => (
        thread_local! {
            pub static $name: DynamicStat<($( $type, )+), BoxCounter> = {
                $crate::__define_key_generator!(
                    __key_generator($prefix, $key; $( $placeholder: $type ),+)
                );

                fn __stat_generator(key: &str) -> BoxCounter {
                    TL_STATS.with(|stats| {
                        stats.create_counter(key)
                    })
                }

                DynamicStat::new(__key_generator, __stat_generator)
            }
        }
    );

    ($prefix:expr;
     $name:ident: dynamic_timeseries($key:expr, ($( $placeholder:ident: $type:ty ),+);
                                     $( $aggregation_type:expr ),*)) => (
        $crate::__define_stat!(
            $prefix;
            $name: dynamic_timeseries(
                $key,
                ($( $placeholder: $type ),+);
                $( $aggregation_type ),* ;
            )
        );
    );

    ($prefix:expr;
     $name:ident: dynamic_timeseries($key:expr, ($( $placeholder:ident: $type:ty ),+);
                                     $( $aggregation_type:expr ),* ; $( $interval:expr ),*)) => (
        thread_local! {
            pub static $name: DynamicStat<($( $type, )+), BoxTimeseries> = {
                $crate::__define_key_generator!(
                    __key_generator($prefix, $key; $( $placeholder: $type ),+)
                );

                fn __stat_generator(key: &str) -> BoxTimeseries {
                    TL_STATS.with(|stats| {
                        stats.create_timeseries(key, &[$( $aggregation_type ),*], &[$( $interval ),*])
                    })
                }

                DynamicStat::new(__key_generator, __stat_generator)
            };
        }
    );

    ($prefix:expr;
     $name:ident: dynamic_histogram($key:expr, ($( $placeholder:ident: $type:ty ),+);
                                    $bucket_width:expr,
                                    $min:expr,
                                    $max:expr
                                    $(, $aggregation_type:expr )*
                                    $(; P $percentile:expr )*)) => (
        thread_local! {
            pub static $name: DynamicStat<($( $type, )+), BoxHistogram> = {
                $crate::__define_key_generator!(
                    __key_generator($prefix, $key; $( $placeholder: $type ),+)
                );

                fn __stat_generator(key: &str) -> BoxHistogram {
                    TL_STATS.with(|stats| {
                        stats.create_histogram(key,
                                               &[$( $aggregation_type ),*],
                                               BucketConfig {
                                                   width: $bucket_width,
                                                   min: $min,
                                                   max: $max,
                                               },
                                               &[$( $percentile ),*])
                    })
                }

                DynamicStat::new(__key_generator, __stat_generator)
            };
        }
    );
}

#[doc(hidden)]
#[macro_export]
macro_rules! __create_stat_key {
    ($prefix:expr, $key:expr) => {{
        use std::borrow::Cow;
        if $prefix.is_empty() {
            Cow::Borrowed($key)
        } else {
            Cow::Owned(format!("{}.{}", $prefix, $key))
        }
    }};
}

/// Define a group of stats with dynamic names all parameterized by the same set of parameters.
/// The intention is that when setting up a structure for some entity with associated stats, then
/// the type produced by this macro can be included in that structure, and initialized with the
/// appropriate name(s). This is more efficient than using single static "dynamic_" versions of
/// the counters.
///
/// ```
/// use stats::prelude::*;
///
/// define_stats_struct! {
///    // struct name, key prefix template, key template params
///    MyThingStat("things.{}.{}", mything_name: String, mything_idx: usize),
///    cache_miss: counter() // default name from the field
/// }
///
/// struct MyThing {
///    stats: MyThingStat,
/// }
///
/// impl MyThing {
///    fn new(somename: String, someidx: usize) -> Self {
///        MyThing {
///           stats: MyThingStat::new(somename, someidx),
///           //...
///        }
///    }
/// }
/// #
/// # fn main() {}
/// ```
#[macro_export]
macro_rules! define_stats_struct {
    // Handle trailing comma
    ($name:ident ($key:expr, $($pr_name:ident: $pr_type:ty),*) ,
        $( $stat_name:ident: $stat_type:tt($( $params:tt )*) , )+) => {
        define_stats_struct!($name ( $key, $($pr_name: $pr_type),*),
            $($stat_name: $stat_type($($params)*)),* );
    };

    // Handle no params
    ($name:ident ($key:expr) ,
        $( $stat_name:ident: $stat_type:tt($( $params:tt )*) ),*) => {
        define_stats_struct!($name ( $key, ),
            $($stat_name: $stat_type($($params)*)),* );
    };
    ($name:ident ($key:expr) ,
        $( $stat_name:ident: $stat_type:tt($( $params:tt )*) , )+) => {
        define_stats_struct!($name ( $key, ),
            $($stat_name: $stat_type($($params)*)),* );
    };

    // Define struct and its methods.
    ($name:ident ($key:expr, $($pr_name:ident: $pr_type:ty),*) ,
        $( $stat_name:ident: $stat_type:tt($( $params:tt )*) ),*) => {
        #[allow(missing_docs)]
        pub struct $name {
            $(pub $stat_name: $crate::__struct_field_type!($stat_type), )*
        }
        impl $name {
            #[allow(unused_imports, missing_docs)]
            pub fn new($($pr_name: $pr_type),*) -> $name {
                use $crate::macros::common_macro_prelude::*;

                lazy_static! {
                    static ref STATS_MAP: Arc<ThreadMap<BoxStatsManager>> = create_map();
                }

                thread_local! {
                    static TL_STATS: PerThread<BoxStatsManager> =
                        STATS_MAP.register(create_stats_manager());
                }

                let prefix = format!($key, $($pr_name),*);

                $name {
                    $($stat_name: $crate::__struct_field_init!(prefix, $stat_name, $stat_type, $($params)*)),*
                }
            }
        }
        impl std::fmt::Debug for $name {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(fmt, "<{}>", stringify!($name))
            }
        }
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __struct_field_type {
    (singleton_counter) => {
        $crate::macros::common_macro_prelude::BoxSingletonCounter
    };
    (counter) => {
        $crate::macros::common_macro_prelude::BoxCounter
    };
    (timeseries) => {
        $crate::macros::common_macro_prelude::BoxTimeseries
    };
    (histogram) => {
        $crate::macros::common_macro_prelude::BoxHistogram
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __struct_field_init {
    ($prefix:expr, $name:ident, singleton_counter, ) => {
        $crate::__struct_field_init! ($prefix, $name, singleton_counter, stringify!($name))
    };
    ($prefix:expr, $name:ident, singleton_counter, $key:expr) => {
        $crate::__struct_field_init! ($prefix, $name, singleton_counter, $key ; )
    };
    ($prefix:expr, $name:ident, singleton_counter, $key:expr ; ) => {{
        let key = format!("{}.{}", $prefix, $key);
        create_singleton_counter(key)
    }};

    ($prefix:expr, $name:ident, counter, ) => {
        $crate::__struct_field_init! ($prefix, $name, counter, stringify!($name))
    };
    ($prefix:expr, $name:ident, counter, $key:expr) => {
        $crate::__struct_field_init! ($prefix, $name, counter, $key ; )
    };
    ($prefix:expr, $name:ident, counter, $key:expr ; ) => {{
        let key = format!("{}.{}", $prefix, $key);
        TL_STATS.with(|stats| {
            stats.create_counter(&key)
        })
    }};

    ($prefix:expr, $name:ident, timeseries, $( $aggregation_type:expr ),+) => {
        $crate::__struct_field_init! ($prefix, $name, timeseries, stringify!($name) ; $($aggregation_type),*)
    };
    ($prefix:expr, $name:ident, timeseries, $key:expr ; $( $aggregation_type:expr ),* ) => {{
        $crate::__struct_field_init! ($prefix, $name, timeseries, $key ; $($aggregation_type),* ;)
    }};
    ($prefix:expr, $name:ident, timeseries, $key:expr ; $( $aggregation_type:expr ),* ; $( $interval:expr ),* ) => {{
        let key = format!("{}.{}", $prefix, $key);
        TL_STATS.with(|stats| {
            stats.create_timeseries(&key, &[$( $aggregation_type ),*], &[$( $interval),*])
        })
    }};

    ($prefix:expr, $name:ident, histogram,
        $bucket_width:expr, $min:expr, $max:expr $(, $aggregation_type:expr)*
        $(; P $percentile:expr )*) => {
        $crate::__struct_field_init! ($prefix, $name, histogram,
            stringify!($name) ; $bucket_width, $min, $max $(, $aggregation_type)*
            $(; P $percentile)* )
    };
    ($prefix:expr, $name:ident, histogram, $key:expr ;
        $bucket_width:expr, $min:expr, $max:expr $(, $aggregation_type:expr)*
        $(; P $percentile:expr )*) => {{
        let key = format!("{}.{}", $prefix, $key);
        TL_STATS.with(|stats| {
            stats.create_histogram(
                &key,
                &[$( $aggregation_type ),*],
                BucketConfig {
                    width: $bucket_width,
                    min: $min,
                    max: $max,
                },
                &[$( $percentile ),*])
        })
    }};
}
