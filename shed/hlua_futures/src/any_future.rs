/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! Support for interactions between Lua coroutines and Rust futures.

use std::result;

use futures::{Future, Poll};
use hlua::{implement_lua_push, AnyLuaValue, AsMutLua, LuaError, PushGuard};

use crate::utils;

/// A future that wraps another future which returns lua's types. It implements
/// [hlua::Push] so the wrapped future can be given to a lua context or be
/// returned from a function.
///
/// TODO: is LuaError the right error type to return here?
pub struct AnyFuture(Box<dyn Future<Item = AnyLuaValue, Error = LuaError> + Send>);

impl AnyFuture {
    /// Create an instance of [AnyFuture] wrapping the provided future
    pub fn new<F>(future: F) -> Self
    where
        F: Future<Item = AnyLuaValue, Error = LuaError> + Send + 'static,
    {
        AnyFuture(Box::new(future))
    }

    /// Read off an AnyFuture from the top of this Lua stack.
    ///
    /// TODO: involve PushGuard in the signature somehow, maybe.
    pub fn from_lua_stack<'lua, L>(lua: PushGuard<L>) -> result::Result<Self, PushGuard<L>>
    where
        L: AsMutLua<'lua>,
    {
        utils::pop_lua_stack(lua)
    }
}

impl Future for AnyFuture {
    type Item = AnyLuaValue;
    type Error = LuaError;

    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

implement_lua_push!(AnyFuture, |_| {});
