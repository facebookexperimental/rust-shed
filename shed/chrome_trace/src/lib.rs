/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Crate for writing Trace Event JSON files that can be read by Google
//! Chrome's Trace Event Profiling Tool (about:tracing). The format of these
//! files is documented [in this document][1]
//!
//! See [the trace event profiling tool documentation][2] for details on
//! working with Chrome traces.
//!
//! [1]: https://docs.google.com/document/d/1CvAClvFfyA5R-PhYUmn5OOQtYMH4h6I0nSsKchNAySU/preview
//! [2]: http://dev.chromium.org/developers/how-tos/trace-event-profiling-tool

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::Path;
#[cfg(not(target_os = "linux"))]
use std::thread::ThreadId;
use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use bytes::Bytes;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

/// Type alias for the [Event::args] field.
pub type Args = HashMap<String, Value>;

/// Phases from the table of "all event types and their associated phases" used
/// in [Event::ph] field.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Phase {
    /// Supported for event type "Duration"
    #[serde(rename = "B")]
    Begin,
    /// Supported for event type "Duration"
    #[serde(rename = "E")]
    End,
    /// Supported for event type "Complete"
    #[serde(rename = "X")]
    Complete,
    /// Supported for event type "Instant"
    #[serde(rename = "I")]
    Instant,
    /// Supported for event type "Async"
    #[serde(rename = "b")]
    AsyncBegin,
    /// Supported for event type "Async"
    #[serde(rename = "e")]
    AsyncEnd,
    /// Supported for event type "Async"
    #[serde(rename = "n")]
    AsyncInstant,
    /// Supported for event type "FLow"
    #[serde(rename = "s")]
    FlowStart,
    /// Supported for event type "FLow"
    #[serde(rename = "t")]
    FlowStep,
    /// Supported for event type "FLow"
    #[serde(rename = "f")]
    FlowEnd,
    /// Supported for event type "Object"
    #[serde(rename = "N")]
    ObjectCreate,
    /// Supported for event type "Object"
    #[serde(rename = "O")]
    ObjectSnapshot,
    /// Supported for event type "Object"
    #[serde(rename = "D")]
    ObjectDestroy,
    /// Supported for event type "Metadata"
    #[serde(rename = "M")]
    Metadata,

    /// The Unspecified variant exists soley to implement the Default trait.
    /// It should not be used; attempting to serialize it will result in an error.
    #[serde(skip_serializing)]
    Unspecified,
}

impl Default for Phase {
    fn default() -> Self {
        Phase::Unspecified
    }
}

/// Struct representing a JSON record of an individual event in a trace, as
/// decribed in the Trace Event format specification. Field names correspond
/// to those expected in the JSON output.
///
/// Optional fields are annotated with attributes and will be excluded from
/// the resulting JSON.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct Event {
    /// The name of the event, as displayed in Trace Viewer
    pub name: String,
    /// The event categories. This is a comma separated list of categories for
    /// the event. The categories can be used to hide events in the Trace Viewer
    /// UI.
    pub cat: String,
    /// The event type. This is a single character which changes depending on the
    /// type of event being output. The valid values are listed in the table
    /// below. We will discuss each phase type below.
    pub ph: Phase,
    /// The process ID for the process that output this event.
    pub pid: u64,
    /// The thread ID for the thread that output this event.
    pub tid: u64,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "duration::serialize_opt",
        deserialize_with = "duration::deserialize_opt"
    )]
    /// The tracing clock timestamp of the event. The timestamps are provided at
    /// microsecond granularity.
    pub ts: Option<Duration>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "duration::serialize_opt",
        deserialize_with = "duration::deserialize_opt"
    )]
    /// Optional. The thread clock timestamp of the event. The timestamps are
    /// provided at microsecond granularity.
    pub tts: Option<Duration>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    /// Any arguments provided for the event. Some of the event types have
    /// required argument fields, otherwise, you can put any information you wish
    /// in here. The arguments are displayed in Trace Viewer when you view an
    /// event in the analysis section.
    pub args: Args,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// A fixed color name to associate with the event. If provided, cname must
    /// be one of the names listed in trace-viewer's base color scheme's reserved
    /// color names list.
    pub cname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Identifier of the event, used for combining events and generating event
    /// trees.
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// An optional scope string can be specified to avoid id conflicts, in which
    /// case we consider events with the same category, scope, and id as events
    /// from the same event tree.
    pub scope: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "duration::serialize_opt",
        deserialize_with = "duration::deserialize_opt"
    )]
    /// Used in "Complete" events to specify the tracing clock duration of
    /// complete events in microseconds.
    pub dur: Option<Duration>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "duration::serialize_opt",
        deserialize_with = "duration::deserialize_opt"
    )]
    /// Used in "Complete" events to specify the thread clock duration of
    /// complete events in microseconds.
    pub tdur: Option<Duration>,
}

impl Event {
    /// Initialize a new Event with the pid and tid set to those of the current thread.
    /// For convenicence, also set the name and phase since these always need to be set.
    /// Depending on the event phase, other fields may be required; these need to be set
    /// afterward for the event to be valid. In particular, essentially all phases require
    /// that a timestamp (`ts`) be set.
    pub fn new<N: ToString>(name: N, phase: Phase) -> Self {
        Self {
            name: name.to_string(),
            ph: phase,
            pid: getpid(),
            tid: gettid(),
            ..Default::default()
        }
    }

    /// Initialize a new Event corresponding to the current moment in time
    /// (relative to a given starting time specified as `epoch`). The pid and
    /// tid will be set to those of the current thread.
    pub fn now<N: ToString>(name: N, phase: Phase, epoch: &Instant) -> Self {
        Self {
            ts: Some(epoch.elapsed()),
            ..Self::new(name, phase)
        }
    }

    /// Set [Event::name]
    pub fn name<T: ToString>(mut self, name: T) -> Self {
        self.name = name.to_string();
        self
    }

    /// Set [Event::category]
    pub fn category<T: ToString>(mut self, category: T) -> Self {
        self.cat = category.to_string();
        self
    }

    /// Set [Event::phase]
    pub fn phase(mut self, phase: Phase) -> Self {
        self.ph = phase;
        self
    }

    /// Set [Event::pid]
    pub fn pid(mut self, pid: u64) -> Self {
        self.pid = pid;
        self
    }

    /// Set [Event::tid]
    pub fn tid(mut self, tid: u64) -> Self {
        self.tid = tid;
        self
    }

    /// Set [Event::ts]
    pub fn ts(mut self, ts: Duration) -> Self {
        self.ts = Some(ts);
        self
    }

    /// Set [Event::tts]
    pub fn tts(mut self, tts: Duration) -> Self {
        self.tts = Some(tts);
        self
    }

    /// Set [Event::args]
    pub fn args(mut self, args: Args) -> Self {
        self.args = args;
        self
    }

    /// Set [Event::cname]
    pub fn cname<T: ToString>(mut self, cname: T) -> Self {
        self.cname = Some(cname.to_string());
        self
    }

    /// Set [Event::id]
    pub fn id<T: ToString>(mut self, id: T) -> Self {
        self.id = Some(id.to_string());
        self
    }

    /// Set [Event::scope]
    pub fn scope<T: ToString>(mut self, scope: T) -> Self {
        self.scope = Some(scope.to_string());
        self
    }

    /// Set [Event::dur]
    pub fn dur(mut self, dur: Duration) -> Self {
        self.dur = Some(dur);
        self
    }

    /// Set [Event::tdur]
    pub fn tdur(mut self, tdur: Duration) -> Self {
        self.tdur = Some(tdur);
        self
    }
}

/// Struct representing a trace in the "JSON Object Format", as decribed
/// in the Trace Event format specification. Field names correspond to
/// those expected in the JSON output.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Trace {
    /// A trace consists of multiple events.
    pub trace_events: Vec<Event>,
}

impl Trace {
    /// Create a new empty trace
    pub fn new() -> Self {
        Self {
            trace_events: Vec::new(),
        }
    }

    /// Add the event to the trace
    pub fn add_event(&mut self, event: Event) {
        self.trace_events.push(event);
    }

    /// Add multiple events to the trace
    pub fn add_events<I: IntoIterator<Item = Event>>(&mut self, events: I) {
        self.trace_events.extend(events);
    }

    /// Save the trace as plain text json encoded into the given file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut f = File::create(path)?;
        let json = self.to_json_string()?;
        f.write_all(json.as_ref())?;
        Ok(())
    }

    /// Save the trace as gzip compressed json encoded into the given file
    pub fn save_gzip<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut f = File::create(path)?;
        let bytes = self.to_json_gzip()?;
        f.write_all(&bytes)?;
        Ok(())
    }

    /// Save the trace as zstd compressed json encoded into the given file
    pub fn save_zstd<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let mut f = File::create(path)?;
        let bytes = self.to_json_zstd()?;
        f.write_all(&bytes)?;
        Ok(())
    }

    /// Load the trace from a plain text json encoded file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut f = File::open(path)?;
        let mut s = String::new();
        f.read_to_string(&mut s)?;
        Self::parse(&s)
    }

    /// Load the trace from a gzip compressed json encoded file
    pub fn load_gzip<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::parse_gzip(File::open(path)?)
    }

    /// Load the trace from a zstd compressed json encoded file
    pub fn load_zstd<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::parse_zstd(File::open(path)?)
    }
}

macro_rules! json_methods_impl {
    ($t:ty) => {
        impl $t {
            /// Parse json encoded data
            pub fn parse(s: &str) -> Result<Self> {
                Ok(serde_json::from_str(s)?)
            }

            /// Parse json encoded gzip compressed data
            pub fn parse_gzip<R: Read>(bytes: R) -> Result<Self> {
                let mut gz = GzDecoder::new(bytes);
                let mut s = String::new();
                gz.read_to_string(&mut s)?;
                Self::parse(&s)
            }

            /// Parse json encoded zstd compressed data
            pub fn parse_zstd<R: Read>(bytes: R) -> Result<Self> {
                let mut dec = zstd::Decoder::new(bytes)?;
                let mut s = String::new();
                dec.read_to_string(&mut s)?;
                Self::parse(&s)
            }

            /// Encode itself as [Value]
            pub fn to_json(&self) -> Result<Value> {
                Ok(serde_json::to_value(self)?)
            }

            /// Encode itself as plain text json
            pub fn to_json_string(&self) -> Result<String> {
                Ok(serde_json::to_string(self)?)
            }

            /// Encode itself as plain text pretty json
            pub fn to_json_pretty(&self) -> Result<String> {
                Ok(serde_json::to_string_pretty(self)?)
            }

            /// Encode itself as gzip compressed json
            pub fn to_json_gzip(&self) -> Result<Bytes> {
                let mut gz = GzEncoder::new(Vec::new(), Compression::fast());
                serde_json::to_writer(&mut gz, self)?;
                Ok(gz.finish().map(Into::into)?)
            }

            /// Encode itself as zstd compressed json
            pub fn to_json_zstd(&self) -> Result<Bytes> {
                let mut enc = zstd::Encoder::new(Vec::new(), 0)?;
                serde_json::to_writer(&mut enc, self)?;
                Ok(enc.finish().map(Into::into)?)
            }
        }
    };
}

json_methods_impl!(Event);
json_methods_impl!(Trace);

fn getpid() -> u64 {
    unsafe { libc::getpid() as u64 }
}

/// Get an integer ID for the current thread on Linux. This is the
/// system-level thread ID assigned by the kernel's scheduler.
#[cfg(target_os = "linux")]
fn gettid() -> u64 {
    // The gettid(2) system call has no wrapper in glibc (because it
    // isn't portable), so we need to make the system call manually.
    // This is only meaningful on Linux systems.
    unsafe { libc::syscall(libc::SYS_gettid) as u64 }
}

/// Get an integer ID for the current thread on non-Linux platforms.
/// Since there is no good portable API for getting a system-level thread
/// ID, this function instead returns a interger representation of
/// the Rust standard library's std::thead::ThreadId type, which
/// (as of Rust 1.24.0) is completely unrelated to any system level
/// thread ID.
#[cfg(not(target_os = "linux"))]
fn gettid() -> u64 {
    let tid = std::thread::current().id();
    // XXX: As of Rust 1.24.0, a std::thead::ThreadId is simply a wrapper
    // around a u64 counter that is incremented for each new thead.
    // Unfortunately, the standard library does not provide a way to
    // extract this counter from the struct, so we have to resort to
    // transmuting it.
    unsafe { std::mem::transmute::<ThreadId, u64>(tid) }
}

/// Module for serializing and deserializing time::Duration structs as integer values
/// in microseconds, as expected by the trace viewer.
mod duration {
    use std::fmt;
    use std::time::Duration;

    use serde::de;
    use serde::de::Visitor;
    use serde::Deserializer;
    use serde::Serializer;

    use super::as_micros;

    pub fn serialize<S>(value: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(as_micros(value))
    }

    pub fn serialize_opt<S>(value: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let value = value.expect("Serialization should be skipped when value is None");
        serialize(&value, serializer)
    }

    struct DurationVisitor;

    impl<'de> Visitor<'de> for DurationVisitor {
        type Value = Duration;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("deserializes a u64 value into a Duration in microseconds")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Duration, E>
        where
            E: de::Error,
        {
            Ok(Duration::from_micros(value))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_u64(DurationVisitor)
    }

    pub fn deserialize_opt<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserialize(deserializer).map(Some)
    }
}

fn as_micros(dur: &Duration) -> u64 {
    dur.as_secs() * 1_000_000 + dur.subsec_micros() as u64
}

#[cfg(test)]
mod tests {
    use maplit::hashmap;
    use serde_json::json;

    use super::*;

    #[test]
    fn to_json() {
        let event = Event {
            name: "my_event".into(),
            cat: "my_category".into(),
            ph: Phase::Begin,
            ts: Some(Duration::from_micros(123)),
            pid: 1234,
            tid: 5678,
            args: hashmap! {"foo".into() => json!(123)},
            ..Default::default()
        };

        let json = event.to_json().unwrap();
        let expected = json!({
            "name":  "my_event",
            "cat": "my_category",
            "ph": "B",
            "ts": 123,
            "pid": 1234,
            "tid": 5678,
            "args": { "foo": 123 },
        });

        assert_eq!(expected, json);
    }

    #[test]
    fn parse_json() {
        let event = Event {
            name: "my_event".into(),
            cat: "my_category".into(),
            ph: Phase::Begin,
            ts: Some(Duration::from_micros(123)),
            pid: 1234,
            tid: 5678,
            args: hashmap! {"foo".into() => json!(123)},
            ..Default::default()
        };

        let json = "{\"name\":\"my_event\",\"cat\":\"my_category\",\
                    \"ph\":\"B\",\"ts\":123,\"pid\":1234,\"tid\":5678,\
                    \"args\":{\"foo\":123}}";
        let parsed = Event::parse(json).unwrap();

        assert_eq!(parsed, event);
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct DurationWrapper {
        #[serde(
            serialize_with = "duration::serialize",
            deserialize_with = "duration::deserialize"
        )]
        dur: Duration,
    }

    #[test]
    fn serialize_duration() {
        let dur = Duration::from_nanos(123_456);
        let wrapper = DurationWrapper { dur };
        let s = serde_json::to_string(&wrapper).unwrap();
        assert_eq!("{\"dur\":123}", s);

        // The parsed Duration will only be at microsecond resolution.
        let expected = DurationWrapper {
            dur: Duration::from_nanos(123_000),
        };
        let parsed = serde_json::from_str::<DurationWrapper>(&s).unwrap();
        assert_eq!(&expected, &parsed);
    }

    #[test]
    fn save_and_load_file() {
        let mut trace = Trace::new();

        let epoch = Instant::now();
        let begin = Event::now("test", Phase::Begin, &epoch);
        std::thread::sleep(std::time::Duration::from_millis(1));
        let end = Event::now("test", Phase::End, &epoch);

        trace.add_event(begin.clone());
        trace.add_event(end.clone());

        let tmp = tempdir::TempDir::new("trace-event").unwrap();
        let path = tmp.path().join("trace.json");

        trace.save(&path).expect("Failed to save trace");
        let loaded = Trace::load(&path).expect("Failed to load trace");

        // The JSON format stores timestamps as an integer in microseconds, so when
        // the JSON is parsed into a Trace object, we'd expect all of the events to
        // have timestamps with microsecond resolution.
        let mut expected = Trace::new();
        expected.add_event(Event {
            ts: Some(Duration::from_micros(as_micros(&begin.ts.unwrap()))),
            ..begin
        });
        expected.add_event(Event {
            ts: Some(Duration::from_micros(as_micros(&end.ts.unwrap()))),
            ..end
        });

        assert_eq!(expected, loaded);
    }

    #[test]
    fn save_and_load_gzip() {
        let mut trace = Trace::new();

        let epoch = Instant::now();
        let begin = Event::now("test", Phase::Begin, &epoch);
        std::thread::sleep(std::time::Duration::from_millis(1));
        let end = Event::now("test", Phase::End, &epoch);

        trace.add_event(begin.clone());
        trace.add_event(end.clone());

        let tmp = tempdir::TempDir::new("trace-event").unwrap();
        let path = tmp.path().join("trace.json.gz");

        trace.save_gzip(&path).expect("Failed to save trace");
        let loaded = Trace::load_gzip(&path).expect("Failed to load trace");

        // The JSON format stores timestamps as an integer in microseconds, so when
        // the JSON is parsed into a Trace object, we'd expect all of the events to
        // have timestamps with microsecond resolution.
        let mut expected = Trace::new();
        expected.add_event(Event {
            ts: Some(Duration::from_micros(as_micros(&begin.ts.unwrap()))),
            ..begin
        });
        expected.add_event(Event {
            ts: Some(Duration::from_micros(as_micros(&end.ts.unwrap()))),
            ..end
        });

        assert_eq!(expected, loaded);
    }

    #[test]
    fn save_and_load_zstd() {
        let mut trace = Trace::new();

        let epoch = Instant::now();
        let begin = Event::now("test", Phase::Begin, &epoch);
        std::thread::sleep(std::time::Duration::from_millis(1));
        let end = Event::now("test", Phase::End, &epoch);

        trace.add_event(begin.clone());
        trace.add_event(end.clone());

        let tmp = tempdir::TempDir::new("trace-event").unwrap();
        let path = tmp.path().join("trace.json.zst");

        trace.save_zstd(&path).expect("Failed to save trace");
        let loaded = Trace::load_zstd(&path).expect("Failed to load trace");

        // The JSON format stores timestamps as an integer in microseconds, so when
        // the JSON is parsed into a Trace object, we'd expect all of the events to
        // have timestamps with microsecond resolution.
        let mut expected = Trace::new();
        expected.add_event(Event {
            ts: Some(Duration::from_micros(as_micros(&begin.ts.unwrap()))),
            ..begin
        });
        expected.add_event(Event {
            ts: Some(Duration::from_micros(as_micros(&end.ts.unwrap()))),
            ..end
        });

        assert_eq!(expected, loaded);
    }
}
