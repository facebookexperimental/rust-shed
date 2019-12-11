/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! A wrapper on top of `hlua::Lua` that implements coroutine support.

use std::fmt;
use std::mem;
use std::ptr;
use std::result;
use std::sync::atomic::{AtomicIsize, Ordering};

use futures::{Async, Future, Poll};
use hlua::{
    ffi, AsLua, AsMutLua, Lua, LuaContext, LuaError, LuaFunctionCallError, LuaRead, Push, PushGuard,
};
use lazy_static::lazy_static;

use crate::AnyFuture;

lazy_static! {
    // This defines unique indexes to use in the Lua registry. Ideally we'd use luaL_ref for this,
    // but the current version of lua52_sys doesn't expose that function. So instead use a
    // sufficiently high starting index that luaL_ref indexes don't collide with these ones.
    //
    // This could wrap around if enough coroutines are created. luaL_ref maintains a free list,
    // so that should be safer to wraparound bugs.
    static ref NEXT_COROUTINE_IDX: AtomicIsize = AtomicIsize::new(16384);
}

/// A handly builder for creataing a [LuaCoroutine]
#[derive(Debug)]
pub struct LuaCoroutineBuilder<L> {
    lua: L,
    stack_idx: i32,
}

impl<'lua, L> LuaCoroutineBuilder<L>
where
    L: AsMutLua<'lua>,
{
    /// Creates and starts the coroutine with the specified parameters -- equivalent to
    /// `coroutine.create` in Lua.
    pub fn create<V, A, E>(
        self,
        args: A,
    ) -> result::Result<LuaCoroutine<L, V>, LuaFunctionCallError<E>>
    where
        A: for<'r> Push<&'r mut Lua<'lua>, Err = E>,
        V: LuaRead<PushGuard<Lua<'lua>>>,
    {
        // Create a new Lua thread to run this coroutine in.
        let main_state = self.lua.as_lua().state_ptr();
        // TODO: replace this with luaL_ref
        let registry_idx = NEXT_COROUTINE_IDX.fetch_add(1, Ordering::Relaxed) as i32;
        let thread_state = unsafe {
            let thread = ffi::lua_newthread(main_state);
            // This will pop the top value from the main stack, i.e. the new thread that was just
            // created. The net effect is that no values are added or removed from the stack.
            ffi::lua_rawseti(main_state, ffi::LUA_REGISTRYINDEX, registry_idx);

            // The first value on the stack should be the coroutine.
            ffi::lua_pushvalue(main_state, self.stack_idx);
            ffi::lua_xmove(main_state, thread, 1);
            thread
        };

        LuaCoroutine::new(self.lua, thread_state, registry_idx, args)
    }
}

impl<'lua, L> LuaRead<L> for LuaCoroutineBuilder<L>
where
    L: AsMutLua<'lua>,
{
    #[inline]
    fn lua_read_at_position(lua: L, index: i32) -> result::Result<LuaCoroutineBuilder<L>, L> {
        let state = lua.as_lua().state_ptr();
        if unsafe {
            // Lua coroutines look like functions.
            ffi::lua_isfunction(state, index)
        } {
            let stack_idx = if index >= 0 {
                index
            } else {
                // https://www.lua.org/pil/24.2.3.html: "Notice that a negative index -x is
                // equivalent to the positive index gettop - x + 1."
                unsafe { ffi::lua_gettop(state) + index + 1 }
            };
            // It would be nice to not carry 'lua' around, but if it owns a Lua object then we
            // can't risk dropping it.
            Ok(LuaCoroutineBuilder { lua, stack_idx })
        } else {
            Err(lua)
        }
    }
}

unsafe impl<'lua, L> AsLua<'lua> for LuaCoroutineBuilder<L>
where
    L: AsLua<'lua>,
{
    #[inline]
    fn as_lua(&self) -> LuaContext {
        self.lua.as_lua()
    }
}

unsafe impl<'lua, L> AsMutLua<'lua> for LuaCoroutineBuilder<L>
where
    L: AsMutLua<'lua>,
{
    #[inline]
    fn as_mut_lua(&mut self) -> LuaContext {
        self.lua.as_mut_lua()
    }
}

/// A structure that holds Lua coroutine and can be interacted with via it's
/// [futures::Future] trait implementation.
#[derive(Debug)]
pub struct LuaCoroutine<L, V> {
    _main_lua: L,
    // Storing the main state separately works around struct definition lifetime issues:
    // https://users.rust-lang.org/t/12737
    main_state: *mut ffi::lua_State,
    thread_state: *mut ffi::lua_State,
    state: LuaCoroutineState<V>,
    registry_idx: i32,
}

// It is ok for a Lua state to be used by different threads, just not concurrently
// so we implement Send. Without this we wouldn't be able to spawn a LuaCoroutine
unsafe impl<L, V> Send for LuaCoroutine<L, V> {}

impl<L, V> Drop for LuaCoroutine<L, V> {
    /// Remove this thread from the registry (replace it with nil).
    fn drop(&mut self) {
        // TODO: replace with luaL_unref once that is exposed in ffi
        unsafe {
            ffi::lua_pushnil(self.main_state);
            ffi::lua_rawseti(self.main_state, ffi::LUA_REGISTRYINDEX, self.registry_idx)
        }
    }
}

impl<'lua, L, V> Future for LuaCoroutine<L, V>
where
    L: AsMutLua<'lua>,
    V: LuaRead<PushGuard<Lua<'lua>>>,
{
    type Item = V;
    type Error = LuaError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let state = self.state.take();
        let (state, result) = self.poll_next(state);
        self.state = state;
        result
    }
}

impl<'lua, L, V> LuaCoroutine<L, V>
where
    L: AsMutLua<'lua>,
    V: LuaRead<PushGuard<Lua<'lua>>>,
{
    fn new<A, E>(
        main_lua: L,
        thread_state: *mut ffi::lua_State,
        registry_idx: i32,
        args: A,
    ) -> result::Result<LuaCoroutine<L, V>, LuaFunctionCallError<E>>
    where
        A: for<'r> Push<&'r mut Lua<'lua>, Err = E>,
    {
        let main_state = main_lua.as_lua().state_ptr();
        let mut coroutine = LuaCoroutine {
            _main_lua: main_lua,
            main_state,
            thread_state,
            state: LuaCoroutineState::Taken,
            registry_idx,
        };
        coroutine.state = coroutine.resume(args)?;
        Ok(coroutine)
    }

    fn thread_lua(&self) -> Lua<'lua> {
        // Lua threads are garbage collected, so we don't need to explicitly close them.
        unsafe { Lua::from_existing_state(self.thread_state, false) }
    }

    fn poll_next(
        &mut self,
        mut state: LuaCoroutineState<V>,
    ) -> (LuaCoroutineState<V>, Poll<V, LuaError>) {
        use self::LuaCoroutineState::*;

        loop {
            state = match state {
                Waiting(mut future) => {
                    match future.poll() {
                        Ok(Async::NotReady) => {
                            return (Waiting(future), Ok(Async::NotReady));
                        }
                        Ok(Async::Ready(val)) => match self.resume(val) {
                            Ok(new_state) => new_state,
                            Err(err) => {
                                return (Errored, Err(err.into()));
                            }
                        },
                        Err(err) => {
                            // Need to maintain the contract that at the end this leaves one extra
                            // value on the stack which the associated PushGuard cleans up.
                            unsafe { ffi::lua_pushnil(self.thread_state) };
                            return (Errored, Err(err));
                        }
                    }
                }
                Done(val) => {
                    return (Returned, Ok(Async::Ready(val)));
                }
                Errored => panic!("polled Lua coroutine after it returned an error"),
                Returned => panic!("polled Lua coroutine after it returned result"),
                Taken => panic!("polled Lua coroutine while state was taken -- logic error?"),
            };
        }
    }

    fn resume<A, E>(
        &mut self,
        args: A,
    ) -> result::Result<LuaCoroutineState<V>, LuaFunctionCallError<E>>
    where
        A: for<'r> Push<&'r mut Lua<'lua>, Err = E>,
    {
        let (ret, guard) = unsafe {
            // lua_resume pops the function, so we have to make a copy of it
            ffi::lua_pushvalue(self.thread_state, -1);
            let mut thread_lua = self.thread_lua();
            let num_pushed = match args.push_to_lua(&mut thread_lua) {
                Ok(g) => {
                    // lua_resume will pop the arguments, so must forget the PushGuard here.
                    g.forget()
                }
                Err((err, _)) => return Err(LuaFunctionCallError::PushError(err)),
            };
            let ret = ffi::lua_resume(self.thread_state, ptr::null_mut(), num_pushed);

            let guard = PushGuard::new(thread_lua, 1);

            (ret, guard)
        };

        match ret {
            0 => match LuaRead::lua_read(guard) {
                Err(_) => Err(LuaFunctionCallError::LuaError(LuaError::WrongType)),
                Ok(x) => Ok(LuaCoroutineState::Done(x)),
            },
            ffi::LUA_ERRMEM => panic!("lua_pcall returned LUA_ERRMEM"),
            ffi::LUA_ERRRUN => {
                let error_msg: String = LuaRead::lua_read(guard)
                    .expect("can't find error message at the top of the Lua stack");
                Err(LuaFunctionCallError::LuaError(LuaError::ExecutionError(
                    error_msg,
                )))
            }
            ffi::LUA_YIELD => {
                // XXX Assume exactly one value for now. How do we know the number of values?
                let future: AnyFuture = match AnyFuture::from_lua_stack(guard) {
                    Ok(val) => val,
                    Err(_) => return Err(LuaFunctionCallError::LuaError(LuaError::WrongType)),
                };
                Ok(LuaCoroutineState::Waiting(future))
            }
            _ => panic!("Unknown error code returned by lua_resume: {}", ret),
        }
    }
}

enum LuaCoroutineState<V> {
    Waiting(AnyFuture),
    Done(V),
    Errored,
    Returned,
    Taken,
}

impl<V> LuaCoroutineState<V> {
    fn take(&mut self) -> LuaCoroutineState<V> {
        mem::replace(self, LuaCoroutineState::Taken)
    }
}

impl<V> fmt::Debug for LuaCoroutineState<V>
where
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LuaCoroutineState::Waiting(_) => f.write_str("Waiting"),
            LuaCoroutineState::Done(ref val) => write!(f, "Done({:?})", val),
            LuaCoroutineState::Errored => write!(f, "Errored"),
            LuaCoroutineState::Returned => f.write_str("Returned"),
            LuaCoroutineState::Taken => f.write_str("Taken"),
        }
    }
}
