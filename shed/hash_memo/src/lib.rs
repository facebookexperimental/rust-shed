/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(missing_docs)]

//! Memoize `Hasher::finish()` values to save recomputing them

use once_cell::sync::OnceCell;
use std::borrow::Borrow;
use std::fmt;
use std::hash::BuildHasher;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Deref;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

/// `BuildMemoHasher` provides a way to construct a wrapper `MemoHasher` of a
/// `std::hash::Hasher`s so that the memoized `Hasher::finish()` values
/// from `EagerHashMemoizer` and `LazyHashMemoizer` are passed through
/// identical to the non-memoized values.  This is useful if you are going to
/// look up a map by both the wrapped memoized value and via
/// `std::borrow::Borrow::borrow()` to `&T`.
#[derive(Clone)]
pub struct BuildMemoHasher<I: BuildHasher> {
    inner_factory: I,
}

impl<I: BuildHasher> BuildMemoHasher<I> {
    /// Make a new `BuildMemoHasher`
    pub fn new(inner_factory: I) -> Self {
        Self { inner_factory }
    }
}

impl<I: BuildHasher + Default> Default for BuildMemoHasher<I> {
    fn default() -> Self {
        Self::new(I::default())
    }
}

impl<I: BuildHasher> BuildHasher for BuildMemoHasher<I> {
    type Hasher = MemoHasher<I>;

    fn build_hasher(&self) -> MemoHasher<I> {
        MemoHasher::new(self.inner_factory.build_hasher())
    }
}

/// A `BuildHasher` that stores a memo'd hash between write and finish
pub struct MemoHasher<I: BuildHasher> {
    inner_hasher: I::Hasher,
    finish_memo: Option<u64>,
    needs_finish: AtomicBool,
    accept_memo: AtomicBool,
}

impl<I: BuildHasher> MemoHasher<I> {
    /// Make a new `MemoHasher`
    pub fn new(inner: I::Hasher) -> Self {
        Self {
            inner_hasher: inner,
            finish_memo: None,
            needs_finish: AtomicBool::new(false),
            accept_memo: AtomicBool::new(false),
        }
    }
}

impl<I: BuildHasher> Hasher for MemoHasher<I> {
    fn finish(&self) -> u64 {
        let v = match self.finish_memo {
            Some(v) => v,
            None => {
                if self.needs_finish.load(Ordering::Acquire) {
                    self.inner_hasher.finish()
                } else {
                    // We have not received the memo or any data yet
                    // This finish was only called so we could start
                    // recording memos
                    self.accept_memo.store(true, Ordering::Release);
                    0
                }
            }
        };
        self.needs_finish.store(false, Ordering::Release);
        v
    }

    fn write_u64(&mut self, v: u64) {
        if self.accept_memo.load(Ordering::Acquire) {
            self.finish_memo = Some(v);
        } else {
            self.needs_finish.store(true, Ordering::Release);
            self.inner_hasher.write_u64(v);
        }
    }

    // Have to override every method on Hasher incase our inner
    // hasher did as well.
    fn write(&mut self, bytes: &[u8]) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write(bytes)
    }

    fn write_u8(&mut self, i: u8) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_u8(i)
    }
    fn write_u16(&mut self, i: u16) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_u16(i)
    }
    fn write_u32(&mut self, i: u32) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_u32(i)
    }
    fn write_u128(&mut self, i: u128) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_u128(i)
    }
    fn write_usize(&mut self, i: usize) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_usize(i)
    }
    fn write_i8(&mut self, i: i8) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_i8(i)
    }
    fn write_i16(&mut self, i: i16) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_i16(i)
    }
    fn write_i32(&mut self, i: i32) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_i32(i)
    }
    fn write_i64(&mut self, i: i64) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_i64(i)
    }
    fn write_i128(&mut self, i: i128) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_i128(i)
    }
    fn write_isize(&mut self, i: isize) {
        self.needs_finish.store(true, Ordering::Release);
        self.inner_hasher.write_isize(i)
    }
}

/// `EagerHashMemoizer` can wrap your type `T` to eagerly memoize your hash
/// result.  This is ideal when you know you are immediately going to use it
/// with something that will call `Hash::hash()`, for example
/// you are going to use is as a key in a `HashMap`.
#[derive(PartialEq, Eq)]
pub struct EagerHashMemoizer<T: Hash> {
    hash_memo: u64,
    inner: T,
}

impl<T: Hash + fmt::Debug> fmt::Debug for EagerHashMemoizer<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EagerHashMemoizer")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<T: Hash> EagerHashMemoizer<T> {
    /// Make a new `EagerHashMemoizer`
    pub fn new<I: BuildHasher>(value: T, factory: &I) -> Self {
        let hash_memo = {
            let mut state = factory.build_hasher();
            value.hash(&mut state);
            state.finish()
        };

        Self {
            hash_memo,
            inner: value,
        }
    }
}

// Memo the hash
impl<T: Hash> Hash for EagerHashMemoizer<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // The empty finish tells MemoHasher to interpret the write_u64 as
        // the finish value.
        // Hasher has only limited apis, and this is safer than transmuting
        // to use a specific write_finish() api.
        state.finish();

        // If we are using the MemoHasher, this will set the finish value.
        // Otherwise it harmlessly alters the hash a bit.
        state.write_u64(self.hash_memo);
    }
}

impl<T: Hash> Borrow<T> for EagerHashMemoizer<T> {
    fn borrow(&self) -> &T {
        &self.inner
    }
}

impl<T: Hash> Deref for EagerHashMemoizer<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// `LazyHashMemoizer` can wrap your type `T` to lazily memoize your
/// hash result. This is useful if you are not sure if `Hash::hash()` will be
/// called (e.g. you might put in a `HashMap`) and you want to defer the cost.
pub struct LazyHashMemoizer<'a, T, I: BuildHasher + 'a> {
    inner: T,
    hash_memo: OnceCell<u64>,
    factory: &'a I,
}

impl<T: fmt::Debug, I: BuildHasher> fmt::Debug for LazyHashMemoizer<'_, T, I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LazyHashMemoizer")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<T: PartialEq, I: BuildHasher> PartialEq for LazyHashMemoizer<'_, T, I> {
    fn eq(&self, other: &LazyHashMemoizer<T, I>) -> bool {
        self.inner == other.inner
    }
}

impl<T: Eq, I: BuildHasher> Eq for LazyHashMemoizer<'_, T, I> {}

impl<'a, T, I: BuildHasher> LazyHashMemoizer<'a, T, I> {
    /// Make a new `LazyHashMemoizer`
    pub fn new(value: T, factory: &'a I) -> Self {
        Self {
            inner: value,
            hash_memo: OnceCell::new(),
            factory,
        }
    }
}

// Memo the hash
impl<T: Hash, I: BuildHasher> Hash for LazyHashMemoizer<'_, T, I> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let inner_hash = self.hash_memo.get_or_init(|| {
            let mut inner_state = self.factory.build_hasher();
            self.inner.hash(&mut inner_state);
            inner_state.finish()
        });

        // Tells MemoHasher to interpret the write_u64 as the finish value.
        // Hasher has only limited apis, and this is safer than transmuting
        // to use a specific write_finish() api.
        state.finish();

        // If we are using the MemoHasher, this will set the finish value.
        // Otherwise it harmlessly alters the hash a bit
        state.write_u64(*inner_hash)
    }
}

impl<T: Hash, I: BuildHasher> Borrow<T> for LazyHashMemoizer<'_, T, I> {
    fn borrow(&self) -> &T {
        &self.inner
    }
}

impl<T, I: BuildHasher> Deref for LazyHashMemoizer<'_, T, I> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ahash::RandomState as AhashRandomState;
    use std::collections::hash_map::RandomState as DefaultRandomState;
    use std::collections::HashMap;

    #[test]
    fn equality_lazy_ahash() {
        equality_lazy(AhashRandomState::default());
    }

    #[test]
    fn equality_lazy_default() {
        equality_lazy(DefaultRandomState::default());
    }
    #[test]
    fn equality_eager_ahash() {
        equality_eager(AhashRandomState::default());
    }

    #[test]
    fn equality_eager_default() {
        equality_eager(DefaultRandomState::default());
    }

    #[allow(clippy::many_single_char_names)]
    fn equality_lazy<I: BuildHasher + Clone>(factory: I) {
        let a = LazyHashMemoizer::new(TestStruct::new("foo", 42), &factory);
        let b = LazyHashMemoizer::new(TestStruct::new("bar", 21), &factory);
        assert_ne!(a, b);

        let c = LazyHashMemoizer::new(TestStruct::new("foo", 42), &factory);
        assert_eq!(a, c);

        // Borrow base case sanity check
        let r = TestStruct::new("foo", 42);
        {
            let borrow: &TestStruct = c.borrow();
            assert_eq!(borrow, &r);
        }

        // Test map of inner works with memo + regular hasher
        {
            let mut m = HashMap::with_hasher(factory.clone());
            // make it big enough there are multiple buckets
            for i in 0..1000 {
                m.insert(TestStruct::new("foo", i), i);
            }
            assert_eq!(Some(&42), m.get(&c));
            assert_eq!(Some(&42), m.get(&r));
            assert_eq!(None, m.get(&b));
        }

        // Test map of memo works with memo + regular hasher
        {
            let mut m =
                HashMap::<LazyHashMemoizer<TestStruct, I>, u32, I>::with_hasher(factory.clone());
            for i in 0..1000 {
                m.insert(
                    LazyHashMemoizer::new(TestStruct::new("foo", i), &factory),
                    i,
                );
            }
            // Fine, even though not using the memo hasher lazy memo lookup is consistent
            assert_eq!(Some(&42), m.get(&c));
            // To have the map borrow lookup work one must be using the MemoHasher
            // Which is why this returns None
            assert_eq!(None, m.get(&r));
            assert_eq!(None, m.get(&b));
        }

        // Test map of inner works with memo + our hasher
        let factory = BuildMemoHasher::new(factory.clone());
        {
            let mut m = HashMap::with_hasher(factory.clone());
            for i in 0..1000 {
                m.insert(TestStruct::new("foo", i), i);
            }
            assert_eq!(Some(&42), m.get(&c));
            assert_eq!(Some(&42), m.get(&r));
            assert_eq!(None, m.get(&b));
        }

        // Test map of memo works with memo + our hasher
        {
            let mut m = HashMap::with_hasher(factory.clone());
            for i in 0..1000 {
                let a = LazyHashMemoizer::new(TestStruct::new("foo", i), &factory);
                m.insert(a.clone(), i);
                assert_eq!(Some(&i), m.get(&a));
            }

            assert_eq!(Some(&42), m.get(&c));
            assert_eq!(Some(&42), m.get(&r));
            assert_eq!(None, m.get(&b));
        }
    }

    #[allow(clippy::many_single_char_names)]
    fn equality_eager<I: BuildHasher + Clone>(factory: I) {
        let a = EagerHashMemoizer::new(TestStruct::new("foo", 42), &factory);
        let b = EagerHashMemoizer::new(TestStruct::new("bar", 21), &factory);
        assert_ne!(a, b);

        let c = EagerHashMemoizer::new(TestStruct::new("foo", 42), &factory);
        assert_eq!(a, c);

        // Borrow base case sanity check
        let r = TestStruct::new("foo", 42);
        {
            let borrow: &TestStruct = c.borrow();
            assert_eq!(borrow, &r);
        }

        // Test map of inner works with memo + regular hasher
        {
            let mut m = HashMap::with_hasher(factory.clone());
            // make it big enough there are multiple buckets
            for i in 0..1000 {
                m.insert(TestStruct::new("foo", i), i);
            }
            assert_eq!(Some(&42), m.get(&c));
            assert_eq!(Some(&42), m.get(&r));
            assert_eq!(None, m.get(&b));
        }

        // Test map of memo works with memo + regular hasher
        {
            let mut m = HashMap::with_hasher(factory.clone());
            for i in 0..1000 {
                let key = EagerHashMemoizer::new(TestStruct::new("foo", i), &factory);
                m.insert(key, i);
            }
            // Fine for eager, as even though not using the memo hasher eager memo lookup is consistent
            // as we always get the same hash
            assert_eq!(Some(&42), m.get(&c));
            // To have the map borrow lookup work one must be using the MemoHasher
            // Which is why this returns None
            assert_eq!(None, m.get(&r));
            assert_eq!(None, m.get(&b));
        }

        // Test map of inner works with memo + our hasher
        let factory = BuildMemoHasher::new(factory);
        {
            let mut m = HashMap::with_hasher(factory.clone());
            for i in 0..1000 {
                m.insert(TestStruct::new("foo", i), i);
            }
            assert_eq!(Some(&42), m.get(&c));
            assert_eq!(Some(&42), m.get(&r));
            assert_eq!(None, m.get(&b));
        }

        // Test map of memo works with memo + our hasher
        {
            let mut m = HashMap::with_hasher(factory.clone());
            for i in 0..1000 {
                let a = EagerHashMemoizer::new(TestStruct::new("foo", i), &factory);
                m.insert(a.clone(), i);
                assert_eq!(Some(&i), m.get(&a));
            }

            assert_eq!(Some(&42), m.get(&c));
            assert_eq!(Some(&42), m.get(&r));
            assert_eq!(None, m.get(&b));
        }
    }

    #[derive(Clone, Debug, Eq, Hash, PartialEq)]
    struct TestStruct {
        name: String,
        id: u32,
    }

    impl TestStruct {
        fn new(name: &str, id: u32) -> Self {
            Self {
                name: name.to_string(),
                id,
            }
        }
    }

    #[test]
    fn hash_memo_vs_inner_ahash() {
        hash_memo_vs_inner(AhashRandomState::default());
    }

    #[test]
    fn hash_memo_vs_inner_default() {
        hash_memo_vs_inner(DefaultRandomState::default());
    }

    #[test]
    fn hash_memo_ahash() {
        hash_memo_for_state(AhashRandomState::default());
    }

    #[test]
    fn hash_memo_default() {
        hash_memo_for_state(DefaultRandomState::default());
    }

    fn hash_memo_vs_inner<I: BuildHasher>(factory: I) {
        let test_value = TestStruct::new("foo", 42);
        // Totally vanilla
        let exemplar = {
            let mut hasher = factory.build_hasher();
            test_value.hash(&mut hasher);
            hasher.finish()
        };

        // Make sure base case is stable
        {
            let new_value = TestStruct::new("foo", 42);
            let mut hasher = factory.build_hasher();
            new_value.hash(&mut hasher);
            assert_eq!(exemplar, hasher.finish());
        }

        // Make sure base case is not matching incorrectly
        {
            let new_value = TestStruct::new("bar", 21);
            let mut hasher = factory.build_hasher();
            new_value.hash(&mut hasher);
            assert_ne!(exemplar, hasher.finish());
        }

        // Check the MemoHasher is stable to exemplar
        {
            let mut hasher = MemoHasher::<I>::new(factory.build_hasher());
            test_value.hash(&mut hasher);
            assert_eq!(exemplar, hasher.finish());
        }
    }

    fn hash_memo_for_state<I: BuildHasher>(factory: I) {
        let test_value = TestStruct::new("foo", 42);

        // Base case is MemoHasher, but no wrapper
        let exemplar = {
            let mut hasher = MemoHasher::<I>::new(factory.build_hasher());
            test_value.hash(&mut hasher);
            hasher.finish()
        };

        // Now introduce our factory
        let factory = BuildMemoHasher::<I>::new(factory);

        // Make sure base case is stable for MemoHasher
        {
            let new_value = TestStruct::new("foo", 42);
            let mut hasher = factory.build_hasher();
            new_value.hash(&mut hasher);
            assert_eq!(exemplar, hasher.finish());
        }

        // Make sure MemoHasher is not matching incorrectly
        {
            let new_value = TestStruct::new("bar", 21);
            let mut hasher = factory.build_hasher();
            new_value.hash(&mut hasher);
            assert_ne!(exemplar, hasher.finish());
        }

        let test_value = EagerHashMemoizer::new(TestStruct::new("foo", 42), &factory);
        // Make sure stable for MemoHasher + EagerHashMemoizer
        {
            let mut hasher = factory.build_hasher();
            test_value.hash(&mut hasher);
            assert_eq!(exemplar, hasher.finish());
        }

        // Make sure stable reuse for MemoHasher + EagerHashMemoizer, with no call to inner_hasher
        {
            let mut hasher = factory.build_hasher();
            test_value.hash(&mut hasher);
            assert_eq!(exemplar, hasher.finish());
        }

        // Make sure MemoHasher + EagerHashMemoizer is not matching incorrectly
        {
            let new_value = EagerHashMemoizer::new(TestStruct::new("bar", 21), &factory);
            let mut hasher = factory.build_hasher();
            new_value.hash(&mut hasher);
            assert_ne!(exemplar, hasher.finish());
        }

        let test_value = LazyHashMemoizer::new(TestStruct::new("foo", 42), &factory);
        // Make sure stable for MemoHasher + LazyHashMemoizer
        {
            let mut hasher = factory.build_hasher();
            test_value.hash(&mut hasher);
            assert_eq!(exemplar, hasher.finish());
        }

        // Make sure stable reuse for MemoHasher + LazyHashMemoizer, with no call to inner_hasher
        {
            let mut hasher = factory.build_hasher();
            test_value.hash(&mut hasher);
            assert_eq!(exemplar, hasher.finish());
        }

        // Make sure MemoHasher + LazyHashMemoizer is not matching incorrectly
        {
            let new_value = LazyHashMemoizer::new(TestStruct::new("bar", 21), &factory);
            let mut hasher = factory.build_hasher();
            new_value.hash(&mut hasher);
            assert_ne!(exemplar, hasher.finish());
        }
    }
}
