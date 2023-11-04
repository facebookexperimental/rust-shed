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

use anyhow::anyhow;
use anyhow::Result;
use futures::future::poll_fn;
use futures::Future;
use futures::FutureExt;

use crate::JustKnobs;

thread_local! {
    static JUST_KNOBS: RefCell<Option<Arc<JustKnobsInMemory>>> = Default::default()
}
pub fn in_use() -> bool {
    JUST_KNOBS.with(|jk| jk.borrow().is_some())
}

#[derive(Default)]
pub struct JustKnobsInMemory(HashMap<String, KnobVal>);
impl JustKnobsInMemory {
    pub fn new(val: HashMap<String, KnobVal>) -> Self {
        JustKnobsInMemory(val)
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
            Some(jk) => *jk.0.get(name).unwrap_or(&KnobVal::Bool(false)),
            None => KnobVal::Bool(false),
        });

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
            Some(jk) => *jk.0.get(name).unwrap_or(&KnobVal::Int(0)),
            None => KnobVal::Int(0),
        });

        match value {
            KnobVal::Bool(_b) => Err(anyhow!(
                "JustKnobs knob {} has type bool while expected int",
                name,
            )),
            KnobVal::Int(v) => Ok(v),
        }
    }
}

/// A helper function to override jk during a closure's execution.
/// This is useful for unit tests.
pub fn with_just_knobs<T>(new_just_knobs: JustKnobsInMemory, f: impl FnOnce() -> T) -> T {
    JUST_KNOBS.with(move |jk| *jk.borrow_mut() = Some(Arc::new(new_just_knobs)));
    let res = f();
    JUST_KNOBS.with(|jk| jk.take());
    res
}

/// A helper function to override jk during a async closure's execution.  This is
/// useful for unit tests.
pub fn with_just_knobs_async<Out, Fut: Future<Output = Out> + Unpin>(
    new_just_knobs: JustKnobsInMemory,
    fut: Fut,
) -> impl Future<Output = Out> {
    with_just_knobs_async_arc(Arc::new(new_just_knobs), fut)
}

pub fn with_just_knobs_async_arc<Out, Fut: Future<Output = Out> + Unpin>(
    new_just_knobs: Arc<JustKnobsInMemory>,
    mut fut: Fut,
) -> impl Future<Output = Out> {
    poll_fn(move |cx| {
        JUST_KNOBS.with(|jk| *jk.borrow_mut() = Some(new_just_knobs.clone()));
        let res = fut.poll_unpin(cx);
        JUST_KNOBS.with(|jk| jk.take());
        res
    })
}

/// A helper function to override jk. Useful for unit tests where we need an override
/// that isn't tied to a single closure/future.
pub fn override_just_knobs(new_just_knobs: Option<JustKnobsInMemory>) {
    JUST_KNOBS.with(|t| *t.borrow_mut() = new_just_knobs.map(Arc::new));
}

#[cfg(test)]
mod test {
    use maplit::hashmap;

    use super::*;

    #[test]
    fn test_jk_override() -> Result<()> {
        assert!(!ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).unwrap());

        let res_closure = with_just_knobs(
            JustKnobsInMemory::new(hashmap! {
                "my/config:knob1".to_string() => KnobVal::Bool(true),
                "my/config:knob2".to_string() => KnobVal::Int(2),
            }),
            || {
                (
                    ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob1", None, None).unwrap(),
                    ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob2", None).unwrap(),
                )
            },
        );
        assert_eq!(res_closure, (true, 2));

        override_just_knobs(Some(JustKnobsInMemory::new(hashmap! {
            "my/config:knob3".to_string() => KnobVal::Int(7),
            "my/config:knob4".to_string() => KnobVal::Bool(true),
        })));
        let res_manual_override = (
            ThreadLocalInMemoryJustKnobsImpl::get("my/config:knob3", None).unwrap(),
            ThreadLocalInMemoryJustKnobsImpl::eval("my/config:knob4", None, None).unwrap(),
        );
        assert_eq!(res_manual_override, (7, true));

        override_just_knobs(None);

        Ok(())
    }
}
