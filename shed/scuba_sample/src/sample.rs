/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! See the [ScubaSample] documentation

use std::collections::hash_map::Entry;
use std::collections::hash_map::HashMap;
use std::num::NonZeroU64;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use sampling::Sampleable;
use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeMap;
use serde_json::Error as SerdeError;
use serde_json::Map;
use serde_json::Number;
use serde_json::Value;
use thiserror::Error;

use crate::value::NullScubaValue;
use crate::value::ScubaValue;

const TIME_COLUMN: &str = "time";
const INT_KEY: &str = "int";
const DOUBLE_KEY: &str = "double";
const NORMAL_KEY: &str = "normal";
const DENORM_KEY: &str = "denorm";
const NORMVECTOR_KEY: &str = "normvector";
const TAGSET_KEY: &str = "tags";
const SUBSET_KEY: &str = "__subset__";

/// The sample that is able to gather values to be written to Scuba.
#[derive(Clone, Debug)]
pub struct ScubaSample {
    time: u64,
    subset: Option<String>,
    values: HashMap<String, ScubaValue>,
}

impl ScubaSample {
    /// Create a new empty sample with the current timestamp as the timestamp of
    /// this sample
    pub fn new() -> Self {
        ScubaSample {
            time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Current timestamp is earlier than UNIX epoch")
                .as_secs(),
            subset: None,
            values: HashMap::new(),
        }
    }

    /// Create a new sample with the provided timestamp, subset and values,
    /// this allows to create immutable objects
    pub fn new_from_values(
        time: u64,
        subset: Option<String>,
        values: HashMap<String, ScubaValue>,
    ) -> Self {
        ScubaSample {
            time,
            subset,
            values,
        }
    }

    /// Joins the values from another scuba sample to the current one.
    /// If a key from the passed in sample is already present in self, the old
    /// value will be overridden
    pub fn join_values(&mut self, sample: &ScubaSample) {
        for (k, v) in sample.values.iter() {
            self.values.insert(k.to_owned(), v.clone());
        }
    }

    /// Joins the values from another scuba sample to the current one. Unlike
    /// [`ScubaSample::join_values`], this panics if there are any values already present in self.
    pub fn join_values_or_panic(&mut self, sample: &ScubaSample) {
        for (k, v) in sample.values.iter() {
            if self.values.insert(k.to_owned(), v.clone()).is_some() {
                panic!("Duplicate value in scuba sample: {k}");
            }
        }
    }

    /// Create a new empty sample with the provided timestamp as the timestamp of
    /// this sample
    pub fn with_timestamp(seconds_since_epoch: u64) -> Self {
        ScubaSample {
            time: seconds_since_epoch,
            subset: None,
            values: HashMap::new(),
        }
    }

    /// Add the provided value to the sample under the provided key.
    /// Overrides the previous value under that key if present.
    pub fn add<K: Into<String>, V: Into<ScubaValue>>(&mut self, key: K, value: V) -> &mut Self {
        self.values.insert(key.into(), value.into());
        self
    }

    /// Add the provided value to the sample under the provided key if value is Some
    /// Overrides the previous value under that key if present.
    pub fn add_opt<K: Into<String>, V: Into<ScubaValue>>(
        &mut self,
        key: K,
        value: Option<V>,
    ) -> &mut Self {
        if let Some(v) = value {
            self.add(key, v);
        }
        self
    }

    /// Return an `Entry` from the internal `HashMap` of sample data under the
    /// provided key.
    pub fn entry<K: Into<String>>(&mut self, key: K) -> Entry<'_, String, ScubaValue> {
        self.values.entry(key.into())
    }

    /// Remove the provided key from the sample data.
    pub fn remove<K: Into<String>>(&mut self, key: K) -> &mut Self {
        self.values.remove(&key.into());
        self
    }

    /// Remove and return the provided key from the sample data.
    pub fn retrieve<K: Into<String>>(&mut self, key: K) -> Option<ScubaValue> {
        self.values.remove(&key.into())
    }

    /// Return reference to the sample data under the provided key or None if not
    /// present in the dataset.
    pub fn get<K: Into<String>>(&self, key: K) -> Option<&ScubaValue> {
        self.values.get(&key.into())
    }

    /// Returns all keys in the sample.
    pub fn keys(&self) -> impl Iterator<Item = String> {
        self.values.keys().cloned()
    }

    /// Set the [subset] of this sample.
    ///
    /// [subset]: https://fburl.com/qa/xqm9hsxx
    pub fn set_subset<S: Into<String>>(&mut self, subset: S) -> &mut Self {
        self.subset = Some(subset.into());
        self
    }

    /// Clear the [subset] of this sample.
    ///
    /// [subset]: https://fburl.com/qa/xqm9hsxx
    pub fn clear_subset(&mut self) -> &mut Self {
        self.subset = None;
        self
    }

    /// Reset the time of this sample with the provided value.
    pub fn set_time(&mut self, time_in_seconds: u64) -> &mut Self {
        self.time = time_in_seconds;
        self
    }

    /// Reset the time of this sample with the current timestamp.
    pub fn set_time_now(&mut self) -> &mut Self {
        self.time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Current timestamp is earlier than UNIX epoch")
            .as_secs();
        self
    }

    /// Serialize the sample into json compatible with Scuba format.
    pub fn to_json(&self) -> Result<Value, SerdeError> {
        let mut json = Map::new();

        // Insert all of the values for this sample into the appropriate sections of
        // the JSON output. Skip any keys that match the time column name.
        for (key, value) in self.values.iter() {
            if key == TIME_COLUMN {
                continue;
            }

            let section = json_section(value).to_string();

            let object = json.entry(section).or_insert(Value::Object(Map::new()));
            if let Value::Object(ref mut map) = *object {
                map.insert(key.clone(), value.clone().into());
            }
        }

        // Add time column to the int section of the sample.
        {
            let int_section = json
                .entry(INT_KEY.to_string())
                .or_insert(Value::Object(Map::new()));
            if let Value::Object(ref mut map) = *int_section {
                map.insert(
                    TIME_COLUMN.to_string(),
                    Value::Number(Number::from(self.time)),
                );
            }
        }

        // If this sample belongs to a subset, add that to the output.
        if let Some(ref subset) = self.subset {
            json.insert(SUBSET_KEY.to_string(), Value::String(subset.clone()));
        }

        Ok(Value::Object(json))
    }

    /// Serialize the sample directly to Scuba JSON, borrowing its keys and values
    /// instead of constructing an intermediate [`Value`]. Sections and columns
    /// are sorted by name, as in [`Self::to_json`]'s default JSON representation.
    pub fn to_json_string(&self) -> Result<String, SerdeError> {
        serde_json::to_string(&JsonSample(self))
    }
}

fn json_section(value: &ScubaValue) -> &'static str {
    match value {
        ScubaValue::Int(_) | ScubaValue::Null(NullScubaValue::Int) => INT_KEY,
        ScubaValue::Double(_) | ScubaValue::Null(NullScubaValue::Double) => DOUBLE_KEY,
        ScubaValue::Normal(_) | ScubaValue::Null(NullScubaValue::Normal) => NORMAL_KEY,
        #[expect(
            deprecated,
            reason = "Existing samples can still contain denorm columns"
        )]
        ScubaValue::Denorm(_) | ScubaValue::Null(NullScubaValue::Denorm) => DENORM_KEY,
        ScubaValue::NormVector(_) | ScubaValue::Null(NullScubaValue::NormVector) => NORMVECTOR_KEY,
        ScubaValue::TagSet(_) | ScubaValue::Null(NullScubaValue::TagSet) => TAGSET_KEY,
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum JsonValue<'a> {
    Column(&'a ScubaValue),
    Timestamp(u64),
}

struct JsonColumn<'a> {
    section: &'static str,
    key: &'a str,
    value: JsonValue<'a>,
}

struct JsonSection<'a>(&'a [JsonColumn<'a>]);

impl Serialize for JsonSection<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for column in self.0 {
            map.serialize_entry(column.key, &column.value)?;
        }
        map.end()
    }
}

struct JsonSample<'a>(&'a ScubaSample);

impl Serialize for JsonSample<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let sample = self.0;
        let mut columns = Vec::with_capacity(sample.values.len() + 1);
        columns.extend(
            sample
                .values
                .iter()
                .filter(|(key, _)| key.as_str() != TIME_COLUMN)
                .map(|(key, value)| JsonColumn {
                    section: json_section(value),
                    key,
                    value: JsonValue::Column(value),
                }),
        );
        columns.push(JsonColumn {
            section: INT_KEY,
            key: TIME_COLUMN,
            value: JsonValue::Timestamp(sample.time),
        });
        columns.sort_unstable_by(|a, b| (a.section, a.key).cmp(&(b.section, b.key)));

        let mut map = serializer.serialize_map(None)?;
        if let Some(subset) = &sample.subset {
            map.serialize_entry(SUBSET_KEY, subset)?;
        }
        for section in columns.chunk_by(|a, b| a.section == b.section) {
            map.serialize_entry(section[0].section, &JsonSection(section))?;
        }
        map.end()
    }
}

impl Sampleable for ScubaSample {
    fn set_sample_rate(&mut self, sample_rate: NonZeroU64) {
        self.add("sample_rate", sample_rate.get());
    }
}

impl Default for ScubaSample {
    fn default() -> Self {
        Self::new()
    }
}

impl IntoIterator for ScubaSample {
    type Item = (String, ScubaValue);
    type IntoIter = ::std::collections::hash_map::IntoIter<String, ScubaValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

impl<'a> IntoIterator for &'a ScubaSample {
    type Item = (&'a String, &'a ScubaValue);
    type IntoIter = ::std::collections::hash_map::Iter<'a, String, ScubaValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

impl<'a> IntoIterator for &'a mut ScubaSample {
    type Item = (&'a String, &'a mut ScubaValue);
    type IntoIter = ::std::collections::hash_map::IterMut<'a, String, ScubaValue>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter_mut()
    }
}

/// An error returned when attempting to do RFE to ScubaSample conversions.
#[derive(Error, Debug)]
pub enum Error {
    /// Error when a required column is missing from the sample data
    #[error("Could not find '{missing_column}' in columns: {columns}")]
    MissingColumn {
        /// Name of the column that was expected but not found
        missing_column: String,
        /// List of available columns
        columns: String,
    },
    /// got null value where a value was expected to be present
    #[error("got null value where a value was expected to be present")]
    UnexpectedNull(String),
    /// result type and expected type mismatched
    #[error("result type and expected type mismatched")]
    InvalidTypeConversion(String),
    /// error while using custom parse function
    #[error("error while using custom parse function")]
    CustomParseError(String),
}

/// A trait that allows for deriving `From<T>` for [ScubaSample].
///
/// ```ignore
/// use scuba_sample::{ScubaSample, StructuredSample};
///
/// #[derive(StructuredSample)]
/// struct Foo {
///     bar: i32,
/// }
///
/// let sample: ScubaSample = Foo { bar: 4 }.into();
/// ```
pub trait StructuredSample {}

/// A trait that allows for deriving `TryFrom<ScubaSample>` for some struct.
///
/// ```
/// use scuba_sample::ScubaSample;
/// use scuba_sample::TryFromSample;
///
/// #[derive(TryFromSample)]
/// struct Foo {
///     bar: i32,
/// }
///
/// let mut sample = ScubaSample::new();
/// sample.add("bar", 4);
/// let foo: Foo = sample.try_into().unwrap();
/// ```
pub trait TryFromSample {}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use nonzero_ext::nonzero;
    use sampling::SampleResult;
    use sampling::Sampling;
    use serde_json::json;

    use super::*;

    fn assert_json_string_compatible(sample: &ScubaSample) {
        let expected = sample.to_json().unwrap();
        let serialized = sample.to_json_string().unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&serialized).unwrap(),
            expected
        );
        assert_eq!(serialized, expected.to_string());
        assert_eq!(serialized, sample.to_json_string().unwrap());
    }

    #[test]
    #[expect(
        deprecated,
        reason = "The wire format must preserve existing denorm columns"
    )]
    fn json_string_preserves_all_value_types_and_time_collisions() {
        let values = vec![
            ScubaValue::Int(i64::MIN),
            ScubaValue::Int(i64::MAX),
            ScubaValue::Double(-0.0),
            ScubaValue::Double(f64::MIN_POSITIVE),
            ScubaValue::Double(f64::MAX),
            ScubaValue::Double(f64::NAN),
            ScubaValue::Double(f64::INFINITY),
            ScubaValue::Double(f64::NEG_INFINITY),
            ScubaValue::Normal(String::new()),
            ScubaValue::Normal("Unicode: 雪 🦀; escaping: \"\\\n\r\t\0".to_owned()),
            ScubaValue::Denorm("legacy\nvalue".to_owned()),
            ScubaValue::NormVector(vec![]),
            ScubaValue::NormVector(vec!["z".to_owned(), "a".to_owned(), "z".to_owned()]),
            ScubaValue::TagSet(HashSet::new()),
            ScubaValue::TagSet(HashSet::from([
                "雪".to_owned(),
                "".to_owned(),
                "a\"".to_owned(),
                "z".to_owned(),
            ])),
            ScubaValue::Null(NullScubaValue::Int),
            ScubaValue::Null(NullScubaValue::Double),
            ScubaValue::Null(NullScubaValue::Normal),
            ScubaValue::Null(NullScubaValue::Denorm),
            ScubaValue::Null(NullScubaValue::NormVector),
            ScubaValue::Null(NullScubaValue::TagSet),
        ];
        let mut sample = ScubaSample::with_timestamp(u64::MAX);
        sample.set_subset("subset\"\\\n雪");
        sample.add("", "empty column name");
        sample.add("__subset__", "ordinary column");
        for (index, value) in values.iter().enumerate() {
            sample.add(format!("column\"\\\n雪_{index}"), value.clone());
        }
        // Exercise the timestamp's sorted position among integer columns.
        sample.add("a", 1);
        sample.add("z", 2);
        assert_json_string_compatible(&sample);

        for value in values {
            sample.add(TIME_COLUMN, value.clone());
            assert_json_string_compatible(&sample);

            let mut only_time = ScubaSample::with_timestamp(12345);
            only_time.add(TIME_COLUMN, value);
            assert_eq!(
                only_time.to_json_string().unwrap(),
                r#"{"int":{"time":12345}}"#
            );
        }
    }

    #[test]
    fn json_string_preserves_empty_samples_and_subsets() {
        let mut sample = ScubaSample::with_timestamp(0);
        assert_json_string_compatible(&sample);
        sample.set_subset("");
        assert_json_string_compatible(&sample);
        sample.clear_subset();
        assert_eq!(sample.to_json_string().unwrap(), r#"{"int":{"time":0}}"#);
    }

    #[test]
    fn json_string_preserves_wide_samples_and_large_containers() {
        let mut sample = ScubaSample::with_timestamp(1750000000);
        for i in 0..512 {
            sample.add(format!("int_{i:04}"), i);
            sample.add(format!("normal_{i:04}"), format!("value {i}"));
        }
        sample.add("large", "雪\n\"\\".repeat(32768));
        let vector = (0..256)
            .map(|i| format!("item {}", i % 31))
            .collect::<Vec<_>>();
        sample.add("vector", vector.clone());
        sample.add("tags", vector.into_iter().collect::<HashSet<_>>());
        assert_json_string_compatible(&sample);
    }

    #[test]
    fn json_string_order_is_independent_of_insertion_order() {
        let mut first = ScubaSample::with_timestamp(42);
        let mut second = ScubaSample::with_timestamp(42);
        for key in ["a", "time", "z", "b"] {
            first.add(key, key);
        }
        for key in ["b", "z", "time", "a"] {
            second.add(key, key);
        }
        assert_eq!(
            first.to_json_string().unwrap(),
            second.to_json_string().unwrap()
        );
        assert_json_string_compatible(&first);
    }

    #[test]
    fn borrowed_json_propagates_serializer_errors() {
        struct FailingWriter;

        impl std::io::Write for FailingWriter {
            fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("injected write failure"))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }

        let sample = ScubaSample::with_timestamp(42);
        let error = serde_json::to_writer(FailingWriter, &JsonSample(&sample)).unwrap_err();
        assert!(error.is_io());
    }

    /// Test that JSON serialization of a ScubaSample matches the expected format.
    #[test]
    fn to_json() {
        let mut sample = ScubaSample::new();
        let test_vec = vec!["foo", "bar", "foo"];

        sample.set_time(12345);
        sample.add("int1", 1);
        sample.add("int2", 2);
        sample.add("double1", 1.0);
        sample.add("double2", std::f64::consts::PI);
        sample.add("normal1", "The quick brown fox...");
        sample.add(
            "denorm1",
            #[allow(deprecated)]
            ScubaValue::Denorm("...jumps over the lazy dog.".into()),
        );
        sample.add("normvec1", test_vec.clone());
        sample.add("tagset1", test_vec.iter().cloned().collect::<HashSet<_>>());

        let json = sample.to_json().unwrap();
        let expected = json!({
            INT_KEY: {
                "time": 12345,
                "int1": 1,
                "int2": 2,
            },
            DOUBLE_KEY: {
                "double1": 1.0,
                "double2": std::f64::consts::PI,
            },
            NORMAL_KEY: {
                "normal1": "The quick brown fox...",
            },
            DENORM_KEY: {
                "denorm1": "...jumps over the lazy dog.",
            },
            NORMVECTOR_KEY: {
                "normvec1": ["foo", "bar", "foo"],
            },
            TAGSET_KEY: {
                "tagset1": ["bar", "foo"],
            },
        });

        assert_eq!(json, expected);
    }

    /// Test that null values work
    #[test]
    fn to_json_null() {
        let mut sample = ScubaSample::new();

        sample.add("nullvalue", ScubaValue::Null(NullScubaValue::Int));

        let json = sample.to_json().unwrap();
        assert_eq!(json.as_object().unwrap().len(), 1);
        // Time column is automatically added
        assert_eq!(json[INT_KEY].as_object().unwrap().len(), 2);
        assert_eq!(json[INT_KEY]["nullvalue"], Value::Null);
    }

    /// Test that the subset field appears in the JSON when specified.
    #[test]
    fn with_subset() {
        let mut sample = ScubaSample::new();
        sample.set_subset("foobar");
        sample.set_time(0);

        let json = sample.to_json().unwrap();
        let expected = json!({
            INT_KEY: {
                "time": 0,
            },
            SUBSET_KEY: "foobar"
        });

        assert_eq!(json, expected);
    }

    /// Test that if a time value is provided by the user, the value is overwritten with
    /// the time value in the ScubaSample struct when the sample is serialized.
    #[test]
    fn time_value() {
        let mut sample = ScubaSample::with_timestamp(0);
        sample.add("time", 1);

        let json = sample.to_json().unwrap();
        let expected = json!({
            INT_KEY: {
                "time": 0,
            }
        });

        assert_eq!(json, expected);

        // Even if the time column is of a type other than ScubaValue::Int, it should
        // still not show up in any other sections of the JSON output.
        sample.add("time", "foo");

        let json = sample.to_json().unwrap();
        let expected = json!({
            INT_KEY: {
                "time": 0,
            }
        });

        assert_eq!(json, expected);
    }

    /// Test that values of different types with the same key don't result in duplicate
    /// keys across the different sections of the JSON output.
    #[test]
    fn duplicate_keys() {
        let mut sample = ScubaSample::with_timestamp(0);
        let test_vec = vec!["a", "b", "c"];

        sample.add("duplicate", 1);
        sample.add("duplicate", std::f64::consts::PI);
        sample.add("duplicate", test_vec.clone());
        sample.add(
            "duplicate",
            test_vec.iter().cloned().collect::<HashSet<_>>(),
        );
        sample.add("duplicate", "test");

        let json = sample.to_json().unwrap();
        let expected = json!({
            INT_KEY: {
                "time": 0,
            },
            NORMAL_KEY: {
                "duplicate": "test",
            },
        });

        assert_eq!(json, expected);
    }

    /// Unit test for join_values
    #[test]
    fn join_values() {
        let mut sample = ScubaSample::with_timestamp(0);
        let mut sample_to_add = ScubaSample::with_timestamp(1);
        sample.add("you", "won't show up due to how we handle collisions");
        sample.add("can", "put");
        sample_to_add.add("anything", "here");
        sample_to_add.add("you", "really");

        sample.join_values(&sample_to_add);
        let json = sample.to_json().unwrap();

        let expected = json!({
            INT_KEY: {
                "time": 0,
            },
            NORMAL_KEY: {
                "you": "really",
                "can": "put",
                "anything" : "here",
            },
        });

        assert_eq!(json, expected);
    }

    #[test]
    fn test_add_sample_rate() {
        let mut sample = ScubaSample::new();
        let sampling = Sampling::SampledIn(nonzero!(10u64));

        assert_eq!(sampling.apply(&mut sample), SampleResult::Include);
        assert_eq!(sample.get("sample_rate"), Some(&ScubaValue::Int(10)));
    }

    #[test]
    fn test_keys_function() {
        // Test empty sample
        let empty_sample = ScubaSample::new();
        assert_eq!(empty_sample.keys().count(), 0);

        // Test sample with various value types
        let mut sample = ScubaSample::new();
        sample.add("int_key", 42);
        sample.add("string_key", "hello");
        sample.add("double_key", std::f64::consts::PI);
        sample.add("vector_key", vec!["a", "b", "c"]);
        sample.add("null_key", ScubaValue::Null(NullScubaValue::Int));

        assert_eq!(
            sample.keys().collect::<HashSet<_>>(),
            vec![
                "int_key",
                "string_key",
                "double_key",
                "vector_key",
                "null_key",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect::<HashSet<_>>()
        );

        // Test after adding and removing values
        sample.add("new_key", "new_value");
        sample.remove("int_key");

        let updated_keys: HashSet<String> = sample.keys().collect();
        assert!(!updated_keys.contains("int_key"));
        assert!(updated_keys.contains("new_key"));
        assert_eq!(updated_keys.len(), 5);

        // Test that iterator collects a snapshot of keys at the time of creation
        let keys_before_addition = sample.keys().collect::<Vec<_>>();
        sample.add("another_key", "another_value");
        let keys_after_addition = sample.keys();

        assert_eq!(keys_before_addition.len(), 5);
        assert_eq!(keys_after_addition.count(), 6);
    }
}
