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
                panic!("Duplicate value in scuba sample: {}", k);
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
    pub fn entry<K: Into<String>>(&mut self, key: K) -> Entry<String, ScubaValue> {
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

            let section = match value {
                ScubaValue::Int(_) => INT_KEY,
                ScubaValue::Double(_) => DOUBLE_KEY,
                ScubaValue::Normal(_) => NORMAL_KEY,
                #[allow(deprecated)]
                ScubaValue::Denorm(_) => DENORM_KEY,
                ScubaValue::NormVector(_) => NORMVECTOR_KEY,
                ScubaValue::TagSet(_) => TAGSET_KEY,
                ScubaValue::Null(v) => match v {
                    NullScubaValue::Int => INT_KEY,
                    NullScubaValue::Double => DOUBLE_KEY,
                    NullScubaValue::Normal => NORMAL_KEY,
                    #[allow(deprecated)]
                    NullScubaValue::Denorm => DENORM_KEY,
                    NullScubaValue::NormVector => NORMVECTOR_KEY,
                    NullScubaValue::TagSet => TAGSET_KEY,
                },
            }
            .to_string();

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
    /// column expected but not found in scuba query results
    #[error("column expected but not found in scuba query results")]
    MissingColumn(String),
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
}
