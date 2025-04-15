/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::cell::RefCell;
/// JustKnobs implementation that thread-local memory for storage. Meant to be used in unit tests.
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;
use std::thread_local;

use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use futures::Future;
use futures::FutureExt;
use futures::future::poll_fn;
use just_knobs_struct::JustKnobs as JustKnobsStruct;

use crate::JustKnobs;

thread_local! {
    static JUST_KNOBS: RefCell<Option<Arc<JustKnobsInMemory>>> = Default::default()
}
pub fn in_use() -> bool {
    JUST_KNOBS.with(|jk| jk.borrow().is_some())
}

#[derive(Clone, Default)]
pub struct JustKnobsInMemory(HashMap<String, KnobVal>);
impl JustKnobsInMemory {
    pub fn new(val: HashMap<String, KnobVal>) -> Self {
        JustKnobsInMemory(val)
    }

    pub fn from_json(just_knobs_json: &str) -> Result<Self> {
        let just_knobs_struct: JustKnobsStruct = serde_json::from_str(just_knobs_json)?;
        Ok(Self::from(&just_knobs_struct))
    }
}

impl From<&JustKnobsStruct> for JustKnobsInMemory {
    fn from(jk: &JustKnobsStruct) -> Self {
        Self(
            jk.bools
                .iter()
                .map(|(k, v)| (k.clone(), KnobVal::Bool(*v)))
                .chain(jk.ints.iter().map(|(k, v)| (k.clone(), KnobVal::Int(*v))))
                .collect(),
        )
    }
}

#[derive(Copy, Clone)]
pub enum KnobVal {
    Bool(bool),
    Int(i64),
}

pub(crate) struct ThreadLocalInMemoryJustKnobsImpl;
impl JustKnobs for ThreadLocalInMemoryJustKnobsImpl {
    fn eval(name: &str, _hash_val: Option<&str>, _switch_val: Option<&str>) -> Result<bool> {
        let value = JUST_KNOBS.with(|jk| match jk.borrow().deref() {
            Some(jk) => {
                jk.0.get(name)
                    .copied()
                    .ok_or_else(|| anyhow!("Missing just knobs bool: {}", name))
            }
            None => bail!("Thread local JUST_KNOBS is not set"),
        })?;

        match value {
            KnobVal::Int(_v) => Err(anyhow!(
                "JustKnobs knob {} has type int while expected bool",
                name,
            )),
            KnobVal::Bool(b) => Ok(b),
        }
    }

    fn get(name: &str, _switch_val: Option<&str>) -> Result<i64> {
        let value = JUST_KNOBS.with(|jk| match jk.borrow().deref() {
            Some(jk) => {
                jk.0.get(name)
                    .copied()
                    .ok_or_else(|| anyhow!("Missing just knobs int: {}", name))
            }
            None => bail!("Thread local JUST_KNOBS is not set"),
        })?;

        match value {
            KnobVal::Bool(_b) => Err(anyhow!(
                "JustKnobs knob {} has type bool while expected int",
                name,
            )),
            KnobVal::Int(v) => Ok(v),
        }
    }
}

/// A helper function to override jk during a closure's execution. Useful for unit tests.
/// JK values that not present in `new_just_knobs` are are not modified.
pub fn with_just_knobs<T>(new_just_knobs: JustKnobsInMemory, f: impl FnOnce() -> T) -> T {
    let old_just_knobs = JUST_KNOBS.with(|jk| jk.take());
    let merged_just_knobs = Arc::new(merge_just_knobs(old_just_knobs.clone(), new_just_knobs));

    JUST_KNOBS.with(|jk| *jk.borrow_mut() = Some(merged_just_knobs));
    let res = f();
    JUST_KNOBS.with(move |jk| *jk.borrow_mut() = old_just_knobs);

    res
}

/// A helper function to override jk during an async closure's execution. Useful for unit tests.
/// JK values that not present in `new_just_knobs` are are not modified.
pub fn with_just_knobs_async<Out, Fut: Future<Output = Out> + Unpin>(
    new_just_knobs: JustKnobsInMemory,
    mut fut: Fut,
) -> impl Future<Output = Out> {
    let old_just_knobs = JUST_KNOBS.with(|jk| jk.borrow().clone());
    let merged_just_knobs = Arc::new(merge_just_knobs(old_just_knobs, new_just_knobs));

    poll_fn(move |cx| {
        let old_just_knobs = JUST_KNOBS.with(|jk| jk.replace(Some(merged_just_knobs.clone())));
        let res = fut.poll_unpin(cx);
        JUST_KNOBS.with(move |jk| *jk.borrow_mut() = old_just_knobs);
        res
    })
}

/// A helper function to override jk. Useful for unit tests where we need an override
/// that isn't tied to a single closure/future.
/// JK values that not present in `new_just_knobs` are not modified.
pub fn override_just_knobs(new_just_knobs: JustKnobsInMemory) {
    JUST_KNOBS.with(|t| {
        let mut t = t.borrow_mut();
        *t = Some(Arc::new(merge_just_knobs(t.take(), new_just_knobs)));
    });
}

/// A helper function to merge in new jk values. Old jk values that are not present
/// in `new_just_knobs` are preserved, while values that are present are overridden.
fn merge_just_knobs(
    old_just_knobs: Option<Arc<JustKnobsInMemory>>,
    new_just_knobs: JustKnobsInMemory,
) -> JustKnobsInMemory {
    let old_just_knobs = match old_just_knobs {
        Some(old_just_knobs) => old_just_knobs,
        None => return new_just_knobs,
    };

    let mut merged_just_knobs = JustKnobsInMemory::clone(&old_just_knobs);
    for (jk_key, jk_value) in new_just_knobs.0 {
        merged_just_knobs.0.insert(jk_key, jk_value);
    }

    merged_just_knobs
}

#[cfg(test)]
mod test {
    use maplit::hashmap;

    use super::*;

    #[test]
    fn test_with_just_knobs() {
        assert!(ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).is_err());

        override_just_knobs(JustKnobsInMemory::new(hashmap! {
            "my/config:knob1".to_string() => KnobVal::Bool(false),
            "my/config:knob3".to_string() => KnobVal::Int(3),
        }));

        assert!(!ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).unwrap());
        assert_eq!(
            ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob3", None).unwrap(),
            3,
        );

        with_just_knobs(
            JustKnobsInMemory::new(hashmap! {
                "my/config:knob1".to_string() => KnobVal::Bool(true),
                "my/config:knob2".to_string() => KnobVal::Int(2),
            }),
            || {
                assert!(
                    ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).unwrap(),
                );
                assert!(
                    ThreadLocalInMemoryJustKnobsImpl::eval(
                        "my/non_existing_config:knob1",
                        None,
                        None
                    )
                    .is_err(),
                );
                assert_eq!(
                    ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob2", None).unwrap(),
                    2
                );
                assert_eq!(
                    ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob3", None).unwrap(),
                    3
                );
                assert!(
                    ThreadLocalInMemoryJustKnobsImpl::get("my/non_existing_config:knob2", None)
                        .is_err(),
                );
            },
        );
    }

    #[tokio::test]
    async fn test_with_just_knobs_async() {
        assert!(ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).is_err());

        override_just_knobs(JustKnobsInMemory::new(hashmap! {
            "my/config:knob1".to_string() => KnobVal::Bool(false),
            "my/config:knob3".to_string() => KnobVal::Int(3),
        }));

        assert!(!ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).unwrap());
        assert_eq!(
            ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob3", None).unwrap(),
            3,
        );

        with_just_knobs_async(
            JustKnobsInMemory::new(hashmap! {
                "my/config:knob1".to_string() => KnobVal::Bool(true),
                "my/config:knob2".to_string() => KnobVal::Int(2),
            }),
            async {
                assert!(
                    ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).unwrap(),
                );
                assert_eq!(
                    ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob2", None).unwrap(),
                    2
                );
                assert_eq!(
                    ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob3", None).unwrap(),
                    3
                );
            }
            .boxed(),
        )
        .await;
    }

    #[test]
    fn test_nested_with_just_knobs() {
        with_just_knobs(
            JustKnobsInMemory::new(hashmap! {
                "my/config:knob1".to_string() => KnobVal::Bool(false),
                "my/config:knob2".to_string() => KnobVal::Int(5),
            }),
            || {
                with_just_knobs(
                    JustKnobsInMemory::new(hashmap! {
                        "my/config:knob1".to_string() => KnobVal::Bool(true),
                        "my/config:knob2".to_string() => KnobVal::Int(4),
                    }),
                    || {
                        assert!(
                            ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None)
                                .unwrap(),
                        );
                        assert_eq!(
                            ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob2", None).unwrap(),
                            4
                        );
                    },
                );

                assert!(
                    !ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).unwrap(),
                );
                assert_eq!(
                    ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob2", None).unwrap(),
                    5,
                );
            },
        );
    }

    #[tokio::test]
    async fn test_nested_with_just_knobs_async() {
        with_just_knobs_async(
            JustKnobsInMemory::new(hashmap! {
                "my/config:knob1".to_string() => KnobVal::Bool(false),
                "my/config:knob2".to_string() => KnobVal::Int(5),
            }),
            async {
                with_just_knobs_async(
                    JustKnobsInMemory::new(hashmap! {
                        "my/config:knob1".to_string() => KnobVal::Bool(true),
                        "my/config:knob2".to_string() => KnobVal::Int(4),
                    }),
                    async {
                        assert!(
                            ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None)
                                .unwrap(),
                        );
                        assert_eq!(
                            ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob2", None).unwrap(),
                            4
                        );
                    }
                    .boxed(),
                )
                .await;

                assert!(
                    !ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).unwrap(),
                );
                assert_eq!(
                    ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob2", None).unwrap(),
                    5,
                );
            }
            .boxed(),
        )
        .await;
    }

    #[test]
    fn test_override_just_knobs() {
        override_just_knobs(JustKnobsInMemory::new(hashmap! {
            "my/config:knob3".to_string() => KnobVal::Int(7),
            "my/config:knob4".to_string() => KnobVal::Bool(true),
        }));

        assert_eq!(
            ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob3", None).unwrap(),
            7
        );
        assert!(ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob4", None, None).unwrap());

        override_just_knobs(JustKnobsInMemory::new(hashmap! {
            "my/config:knob4".to_string() => KnobVal::Bool(false),
            "my/config:knob5".to_string() => KnobVal::Int(13),
        }));

        assert_eq!(
            ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob3", None).unwrap(),
            7
        );
        assert!(!ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob4", None, None).unwrap());
        assert_eq!(
            ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob5", None).unwrap(),
            13
        );
    }
}
