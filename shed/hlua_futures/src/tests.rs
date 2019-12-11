/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! Test that a FutureResult with an OK value is read properly.

use assert_matches::assert_matches;
use futures::{future, Future};
use hlua::{self, AnyLuaValue, Lua, LuaError};

use crate::{AnyFuture, LuaCoroutine, LuaCoroutineBuilder};

const TEST_CODE: &str = "
function test_ok(s)
    x = coroutine.yield(ok_future())
    return x == s
end

function test_err(s)
    x = coroutine.yield(err_future())
    return x == s
end
";

#[test]
fn test_ok() {
    let mut lua = new_lua();

    {
        let builder: LuaCoroutineBuilder<_> =
            lua.get("test_ok").expect("function test_ok not found");
        let coroutine: LuaCoroutine<_, bool> =
            builder.create("ok").expect("coroutine creation failed");
        let result = coroutine.wait().expect("unexpected error");
        assert!(result);
    }

    // Try running the same coroutine again.
    {
        let builder: LuaCoroutineBuilder<_> =
            lua.get("test_ok").expect("function test_ok not found");
        let coroutine: LuaCoroutine<_, bool> =
            builder.create("ok").expect("coroutine creation failed");
        let result = coroutine.wait().expect("unexpected error");
        assert!(result);
    }

    // Run it with an incorrect value -- try using into_get to ensure that `lua` doesn't get
    // dropped too soon.
    {
        let builder: LuaCoroutineBuilder<_> = match lua.into_get("test_ok") {
            Ok(builder) => builder,
            Err(_) => panic!("function test_ok not found"),
        };
        let coroutine: LuaCoroutine<_, bool> =
            builder.create("not ok").expect("coroutine creation failed");
        let result = coroutine.wait().expect("unexpected error");
        assert!(!result);
    }
}

#[test]
fn test_err() {
    let mut lua = new_lua();

    {
        let builder: LuaCoroutineBuilder<_> =
            lua.get("test_err").expect("function test_err not found");
        let coroutine: LuaCoroutine<_, bool> = builder
            .create("test123")
            .expect("coroutine creation failed");
        let result = coroutine.wait().expect_err("unexpected Ok");
        assert_matches!(result, LuaError::ExecutionError(ref x) if x == "FAIL");
    }

    {
        let builder: LuaCoroutineBuilder<_> =
            lua.get("test_err").expect("function test_err not found");
        let coroutine: LuaCoroutine<_, bool> =
            builder.create("wat").expect("coroutine creation failed");
        let result = coroutine.wait();
        let result = result.expect_err("unexpected Ok");
        assert_matches!(result, LuaError::ExecutionError(ref x) if x == "FAIL");
    }
}

fn new_lua<'lua>() -> Lua<'lua> {
    let mut lua = Lua::new();
    lua.openlibs();
    lua.execute::<()>(TEST_CODE)
        .expect("test code failed to execute");

    lua.set("ok_future", hlua::function0(ok_future));
    lua.set("err_future", hlua::function0(err_future));
    lua
}

fn ok_future() -> AnyFuture {
    AnyFuture::new(future::ok(AnyLuaValue::LuaString("ok".into())))
}

fn err_future() -> AnyFuture {
    AnyFuture::new(future::err(LuaError::ExecutionError("FAIL".into())))
}
