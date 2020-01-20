/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use crate::context::TraceContext;
use chrome_trace::Args;
use futures::task_local;
use lazy_static::lazy_static;
use std::sync::atomic::{AtomicUsize, Ordering};

mod future;
mod stream;

lazy_static! {
    static ref TASK_COUNTER: AtomicUsize = AtomicUsize::new(0);
    static ref EVENT_COUNTER: AtomicUsize = AtomicUsize::new(0);
}

task_local! {
    static TASK_ID: usize = TASK_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Returns a counter representing the ID of the current task. This ID is arbitrary
/// and unrelated to the way Tokio idenfies tasks internally.
fn get_task_id() -> usize {
    TASK_ID.with(|n| *n)
}

/// Returns the value of a monotonically increasing counter, to help give events
/// in the trace unique identifiers.
fn new_event_id() -> usize {
    EVENT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// A type representing an event ID, as used by `traced_with_id`.
#[derive(Clone, Copy)]
pub struct EventId {
    id: usize,
}

impl EventId {
    /// Get a new event ID for `traced_with_id`. Events with the same ID but different names are
    /// nested by the event viewer.
    pub fn new() -> Self {
        Self { id: new_event_id() }
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

/// The `Traced<T>` trait adds the `traced()` method to `Future`s and `Stream`s.
/// The type parameter has no significance beyond the type level, but allows for blanket
/// impls for all types implementing `Future` and `Stream` without causing overlap.
pub trait Traced<T>: Sized {
    /// The type of the wrapped Future or Stream that is being returned from
    /// this trait.
    type Wrapper;

    /// A combinator that returns a wrapper that will add some statistics about
    /// the execution of the wrapped Future or Stream to the given
    /// [TraceContext] as events of name `name` and additional optional `args`
    fn traced<N: ToString>(
        self,
        context: &TraceContext,
        name: N,
        args: Option<Args>,
    ) -> Self::Wrapper;

    /// Similar to [Traced::traced], but also lets you set the EventId
    fn traced_with_id<N: ToString>(
        self,
        context: &TraceContext,
        name: N,
        args: Option<Args>,
        id: EventId,
    ) -> Self::Wrapper;
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::{Duration, Instant};

    use futures::{future, Future, Stream};
    use tokio::{
        self,
        timer::{Delay, Interval},
    };

    use crate::context::TraceContext;

    #[test]
    fn future() {
        let context = TraceContext::default();
        context.enable();

        // Need to use `future::lazy` to ensure that there's no delay between setting the
        // deadline for the Delay and actually starting execution of the Future on tokio's
        // threadpool. Otherwise, the run time reported by the trace will be shorter than
        // expected.
        let sleep = future::lazy(|| Delay::new(Instant::now() + Duration::from_millis(10)))
            .map_err(|_| ())
            .traced(&context, "my_event", None);
        tokio::run(sleep);

        // Check that the future was logged.
        let trace = context.snapshot();
        assert_eq!(2, trace.trace_events.len());
        assert_eq!("my_event", trace.trace_events[0].name);

        // Check that the future ran for as long as expected.
        let start = trace.trace_events[0].ts.unwrap();
        let end = trace.trace_events[1].ts.unwrap();
        println!("{:?}", end - start);
        assert!(end - start >= Duration::from_millis(10));
    }

    #[test]
    fn stream() {
        let context = TraceContext::default();
        context.enable();

        // Need to use `future::lazy` to ensure that there's no delay between setting the
        // deadline for the Interval and actually starting execution of the Stream on tokio's
        // threadpool. Otherwise, the run time reported by the trace will be shorter than
        // expected.
        let sleeps = future::lazy({
            let context = context.clone();
            move || {
                // Set the Interval to start 5ms in the future to ensure that we observe
                // 4 full polls.
                Interval::new(
                    Instant::now() + Duration::from_millis(5),
                    Duration::from_millis(5),
                )
                .take(4)
                .traced(&context, "my_stream", None)
                .collect()
                .then(|_| Ok(()))
            }
        });
        tokio::run(sleeps);

        // Check that the future was logged.
        let trace = context.snapshot();
        assert_eq!(2, trace.trace_events.len());
        assert_eq!("my_stream", trace.trace_events[0].name);

        // Check that the future ran for as long as expected.
        let start = trace.trace_events[0].ts.unwrap();
        let end = trace.trace_events[1].ts.unwrap();
        assert!(end - start >= Duration::from_millis(20));
    }
}
