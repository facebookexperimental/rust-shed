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
        #[cfg(fbcode_build)]
        {
            Command::new(buck_resources::get(format!(
                "common/rust/shed/panichandler/{}",
                $name
            ))?)
        }
        #[cfg(not(fbcode_build))]
        {
            Command::cargo_bin($name)?
        }
    }};
}

#[test]
fn test() -> Result<()> {
    let mut cmd = get_command!("shed_panic_simple");
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
    let mut cmd = get_command!("shed_panic_deep");
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
    let mut cmd = get_command!("shed_panic_multithread");
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
    let mut cmd = get_command!("shed_panic_multithread_abort");
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
