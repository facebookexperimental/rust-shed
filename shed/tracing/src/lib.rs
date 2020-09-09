/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Defines [TraceContext] for storing traces of events and exposing them as
//! data viewable in Chrome trace viewer. It also defines useful trait
//! extensions for easy tracing of [futures::Future] and [futures::Stream].

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

mod r#async;
mod context;
pub mod global;
mod guard;

pub use crate::r#async::{EventId, Traced};
pub use crate::context::{generate_trace_id, TraceContext, TraceId};
pub use crate::global::{disable, enable, is_enabled};
pub use serde_json::Value;

/// Macro for assembling a `HashMap<String, serde_json::Value>` that will be added to
/// a given trace event as its arguments parameter, for display in the trace viewer.
/// A "location" argument containing the name of the source file and the line of the
/// macro invocation is automatically added to make traces for useful. This macro was
/// adapted from the `hashmap!` macro from the `maplit` crate.
#[macro_export]
macro_rules! trace_args {
    (@single $($x:tt)*) => (());
    (@count $($rest:expr),*) => (<[()]>::len(&[$(trace_args!(@single $rest)),*]));

    ($($key:expr => $value:expr,)+) => { trace_args!($($key => $value),+) };
    ($($key:expr => $value:expr),*) => {
        {
            let _cap = trace_args!(@count $($key),*);
            let mut _map = ::std::collections::HashMap::with_capacity(_cap);
            $(
                let key = String::from($key);
                let value = $crate::Value::from($value);
                let _ = _map.insert(key, value);
            )*
            let loc = $crate::Value::from(format!("{}:{}", file!(), line!()));
            let _ = _map.insert("location".into(), loc);
            Some(_map)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{thread, time::Duration};

    #[test]
    fn simple_synchronous_trace() {
        let context = TraceContext::default();
        context.enable();
        {
            let _guard = context.trace("my_event", trace_args!());
            thread::sleep(Duration::from_millis(10));
        }
        let trace = context.snapshot();
        assert_eq!("my_event", trace.trace_events[0].name);
    }

    #[test]
    fn disable_context() {
        let context = TraceContext::default();
        context.disable();
        {
            let _guard = context.trace("my_event", trace_args!());
            thread::sleep(Duration::from_millis(10));
        }
        let trace = context.snapshot();
        assert!(trace.trace_events.is_empty());
    }

    #[test]
    fn sample_rate_zero() {
        global::set_sample_rate(0);
        let context = TraceContext::default();
        {
            let _guard = context.trace("my_event", trace_args!());
            thread::sleep(Duration::from_millis(10));
        }
        let trace = context.snapshot();
        assert!(trace.trace_events.is_empty());
    }

    #[test]
    fn trace_args() {
        let context = TraceContext::default();
        context.enable();
        {
            let mut guard = context.trace(
                "my_event",
                trace_args! {
                    "foo" => "abc",
                    "bar" => 123,
                    "baz" => json!({
                        "a": 1,
                        "b": "2",
                    }),
                },
            );

            let args = guard.args();
            args.insert("foo".into(), 456.into());
            let _ = args
                .remove("location")
                .expect("'location' argument not set");

            thread::sleep(Duration::from_millis(10));
        }
        let trace = context.snapshot();
        let args_json = serde_json::to_value(&trace.trace_events[0].args).unwrap();

        let expected = json!({
            "foo": 456,
            "bar": 123,
            "baz": {
                "a": 1,
                "b": "2",
            }
        });
        assert_eq!(expected, args_json);
    }
}
