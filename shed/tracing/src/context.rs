/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use std::fmt;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Instant;

use parking_lot::Mutex;
use rand::{
    distributions::{Alphanumeric, Distribution, Uniform},
    thread_rng, Rng,
};

use chrome_trace::{Args, Event, Trace};

use crate::global;
use crate::guard::TraceGuard;

/// The `TraceContext` type manages all of the state associated with tracing.
/// Typically, a `TraceContext` will be created early on in program execution
/// (such as in `main()`) and held onto for the duration of the program's lifecycle.
///
/// The context can be cheaply cloned, producing contexts which refer to the same
/// internal state. As such, these contexts can easily be tucked away into any struct
/// or module that requires tracing, and easily propagated throughout the program.
///
/// The primary API for tracing things is the `.trace()`` method on the context.
/// This produces a `TraceGuard`, which is an RAII struct that emits a trace event
/// when it goes out of scope. As such, typical usage of this crate would involve
/// creating a `TraceContext` in `main()`, threading the context through the program,
/// cloning as needed, and then calling `.trace()` in each scope to be traced.
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub(crate) inner: Arc<TraceContextInner>,
}

impl TraceContext {
    /// Initialize a new TraceContext, which manages an individual trace.
    /// The trace will be associated with the given id, and all durations logged
    /// will be relative to the starting time provided as `epoch`.
    pub fn new(id: TraceId, epoch: Instant) -> Self {
        let inner = Arc::new(TraceContextInner::new(id, epoch));

        // Randomly disable a fraction of all contexts, determined by the global sample rate.
        let sample_rate = global::sample_rate();
        if Uniform::new(0usize, 100).sample(&mut thread_rng()) > sample_rate {
            inner.disable();
        }

        Self { inner }
    }

    /// Start tracing a new event. Returns a TraceGuard object that will log a trace event
    /// when it goes out of scope.
    pub fn trace<T: ToString>(&self, name: T, args: Option<Args>) -> TraceGuard {
        TraceGuard::new(&self.inner, name.to_string(), args.unwrap_or_default())
    }

    /// Get a copy of the current trace, including all events that have been logged to
    /// this context up to this point.
    pub fn snapshot(&self) -> Trace {
        self.inner.data.lock().trace.clone()
    }

    /// Get the id of this trace. Useful for logging (e.g., to associate log messages
    /// with a particular trace).
    pub fn id(&self) -> TraceId {
        self.inner.id.clone()
    }

    /// Enable this trace ignoring the [crate::global::is_enabled] and
    /// [crate::global::sample_rate] values
    pub fn enable(&self) {
        self.inner.enable()
    }

    /// Disable this trace.
    pub fn disable(&self) {
        self.inner.disable()
    }

    /// Check if this trace is enabled or not
    pub fn is_enabled(&self) -> bool {
        self.inner.is_enabled()
    }
}

impl Default for TraceContext {
    /// Get a TraceContext with a random id whose start time is set to the time of creation.
    /// This is the recommended way of starting a new trace.
    fn default() -> Self {
        Self::new(generate_trace_id(), Instant::now())
    }
}

/// Identifier of a trace
#[derive(Clone, Debug)]
pub struct TraceId(String);

impl TraceId {
    /// Use the given string as a trace identifier
    pub fn from_string<T: ToString>(s: T) -> Self {
        Self(s.to_string())
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Generate a [TraceId] randomly from a string of 16 alphanumeric characters
pub fn generate_trace_id() -> TraceId {
    TraceId(thread_rng().sample_iter(&Alphanumeric).take(16).collect())
}

#[derive(Debug)]
pub(crate) struct TraceContextInner {
    pub(crate) id: TraceId,
    pub(crate) epoch: Instant,
    pub(crate) enabled: AtomicBool,
    pub(crate) data: Mutex<TraceContextMutableData>,
}

impl TraceContextInner {
    fn new(id: TraceId, epoch: Instant) -> Self {
        Self {
            id,
            epoch,
            enabled: AtomicBool::new(global::is_enabled()),
            data: Mutex::new(TraceContextMutableData::new()),
        }
    }

    pub(crate) fn add_event(&self, event: Event) {
        if !self.is_enabled() {
            return;
        }

        self.data.lock().trace.add_event(event);
    }

    pub(crate) fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed)
    }

    pub(crate) fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed)
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

#[derive(Debug, Default)]
pub(crate) struct TraceContextMutableData {
    pub(crate) trace: Trace,
}

impl TraceContextMutableData {
    fn new() -> Self {
        Default::default()
    }
}
