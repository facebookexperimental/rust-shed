/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! Utility functions for Lua and hlua. Much of this code deals with missing features in the hlua
//! API.

use std::any::{Any, TypeId};
use std::ptr;
use std::result;

use hlua::{ffi, AnyLuaValue, AsLua, AsMutLua, Lua, LuaRead, Push, PushGuard, UserdataOnStack};

/// Dump this Lua stack to stdout. Useful for debugging.
///
/// Some variant of this should probably be upstreamed.
#[allow(dead_code)]
pub fn dump_stack<'lua, L>(lua: L)
where
    L: AsLua<'lua>,
{
    let state = lua.as_lua().state_ptr();
    let top = unsafe { ffi::lua_gettop(state) };
    println!("[state {:?}] total items: {}", state, top);
    for idx in 1..=top {
        let val: AnyLuaValue = match LuaRead::lua_read_at_position(&lua, idx) {
            Ok(val) => val,
            Err(_lua) => continue,
        };
        println!("[state {:?}, at idx {}] {:?}", state, idx, val);
    }
}

/// Dump this Lua stack to stdout. Useful for debugging.
#[allow(dead_code)]
pub fn dump_stack_ptr(state: *mut ffi::lua_State) {
    let lua = unsafe { Lua::from_existing_state(state, false) };
    dump_stack(&lua);
}

extern "C" fn no_destructor(lua: *mut ffi::lua_State) -> libc::c_int {
    unsafe {
        let obj = ffi::lua_touserdata(lua, -1);
        ptr::drop_in_place(obj as *mut TypeId);
        0
    }
}

/// Move a Rust value from the top of the Lua stack onto the Rust stack. This takes a
/// `PushGuard` representing the value on the Lua stack.
pub fn pop_lua_stack<'lua, L, T>(lua: PushGuard<L>) -> result::Result<T, PushGuard<L>>
where
    L: AsMutLua<'lua>,
    T: 'lua + Any,
{
    let mut userdata: UserdataOnStack<T, _> = LuaRead::lua_read(lua)?;

    // Since the struct on the Lua stack will no longer be valid once it's moved, ensure its
    // destructor doesn't run.
    {
        let state_ptr = userdata.as_lua().state_ptr();

        unsafe {
            if ffi::lua_getmetatable(state_ptr, -1) == 0 {
                // Userdatas managed by hlua should always have metatables.
                panic!("hlua userdata has no metatable");
            }
        }

        match "__gc".push_to_lua(&mut userdata) {
            Ok(p) => unsafe { p.forget() },
            Err(_) => unreachable!(),
        };

        unsafe {
            ffi::lua_pushcfunction(state_ptr, no_destructor);
            ffi::lua_settable(state_ptr, -3);
            // We pushed 3 values onto the stack, and settable consumed two of them.
            // Pop the last value off.
            let _guard = PushGuard::new(&mut userdata, 1);
        }
    }

    // *userdata is &T -- copy the bytes onto the Rust stack.
    Ok(unsafe { ptr::read::<T>(&*userdata) })
}
