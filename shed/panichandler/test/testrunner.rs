/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::process::Command;

use anyhow::Result;
use assert_cmd::prelude::*;
use predicates::prelude::*;

macro_rules! get_command {
    ( $name:expr ) => {{
        let maybe_cmd = {
            #[cfg(fbcode_build)]
            {
                facebook::get_command!($name)
            }
            #[cfg(not(fbcode_build))]
            {
                None
            }
        };

        maybe_cmd
            .map(Ok)
            .unwrap_or_else(|| Command::cargo_bin($name))?
    }};
}

#[test]
fn test() -> Result<()> {
    let mut cmd = get_command!("test");
    cmd.assert()
        .failure()
        .code(101)
        .stdout("I'm on an adventure!\n")
        .stderr(predicates::str::starts_with(
            "PANIC: I paniced! Everything's awful! 1234\n",
        ));
    Ok(())
}

#[test]
fn test_deep() -> Result<()> {
    let mut cmd = get_command!("test_deep");
    cmd.assert()
        .failure()
        .code(101)
        .stdout("I'm on an adventure!\n")
        .stderr(
            predicates::str::starts_with("PANIC: I paniced! Everything's awful! 1234\n")
                .and(predicates::str::is_match(r"(limiting \d* frames to 1000)")?),
        );
    Ok(())
}

#[test]
fn testmultithread() -> Result<()> {
    let mut cmd = get_command!("testmultithread");
    cmd.assert()
        .failure()
        .code(99)
        .stdout("I'm on an adventure!\n")
        .stderr(predicates::str::starts_with(
            "PANIC: I paniced! Everything's awful! 1234\n",
        ));
    Ok(())
}

#[test]
fn testmultithread_abort() -> Result<()> {
    let mut cmd = get_command!("testmultithread_abort");
    let assert = cmd.assert();
    let assert = if cfg!(windows) {
        assert.failure().code(predicate::ne(101))
    } else {
        assert.interrupted()
    };
    assert
        .stdout("I'm on an adventure!\n")
        .stderr(predicates::str::starts_with(
            "PANIC: I paniced! Everything's awful! 1234\n",
        ));
    Ok(())
}
