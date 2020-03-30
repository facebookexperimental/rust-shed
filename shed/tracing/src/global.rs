/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Define functions that can modify the global state of tracing.

#[cfg(not(test))]
use std::env;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::context::TraceContext;
use lazy_static::lazy_static;

#[cfg(not(test))]
const ENABLE_TRACE_ENV: &str = "RUST_TRACE";

lazy_static! {
    static ref ENABLED: AtomicBool = init_enabled();
    static ref SAMPLE_RATE: AtomicUsize = init_sample_rate();
    static ref TRACE_CONTEXT: TraceContext = TraceContext::default();
}

/// Check if the tracing is globally enabled
#[inline]
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

/// Globally enable tracing
#[inline]
pub fn enable() {
    ENABLED.store(true, Ordering::Relaxed);
}

/// Globally disable tracing
#[inline]
pub fn disable() {
    ENABLED.store(false, Ordering::Relaxed);
}

/// Check the global sample rate for tracing, returned value is a percentage.
#[inline]
pub fn sample_rate() -> usize {
    SAMPLE_RATE.load(Ordering::Relaxed)
}

/// Set a sample rate for tracing where `rate` is the percentage of samples that
/// will be enabled.
///
/// # Panics
///
/// Panics if `rate` is over 100
#[inline]
pub fn set_sample_rate(rate: usize) {
    assert!(rate <= 100);
    SAMPLE_RATE.store(rate, Ordering::Relaxed);
}

#[cfg(test)]
fn init_enabled() -> AtomicBool {
    AtomicBool::new(true)
}

#[cfg(not(test))]
fn init_enabled() -> AtomicBool {
    AtomicBool::new(env::var_os(ENABLE_TRACE_ENV).is_some())
}

fn init_sample_rate() -> AtomicUsize {
    AtomicUsize::new(100)
}

/// Get a global context, useful when debugging a piece of code that does not
/// have an easily accessible TraceContext passed from callsites. It is not
/// recommended to use it in long running processes (like servers) in
/// production as the trace might easily grow to big to be opened by a trace
/// viewer after awhile.
pub fn get_global_context() -> &'static TraceContext {
    &TRACE_CONTEXT
}
