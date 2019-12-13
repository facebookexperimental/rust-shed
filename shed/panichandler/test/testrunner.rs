/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use std::env;
use std::process::Command;

use anyhow::Result;
use assert_cmd::prelude::*;
use predicates::prelude::*;

macro_rules! get_command {
    ( $name:expr ) => {{
        // This disabling logic is helping in fbcode_build builds using Buck
        // because the Command::cargo_bin cannot be used in that context
        // and a workaround have to be provided.
        let disable: bool = {
            let var = env::var("PANICHANDLER_TESTS_DISABLE").ok();
            let var = var
                .as_ref()
                .map(String::as_str)
                .or(option_env!("PANICHANDLER_TESTS_DISABLE"));
            match var {
                Some(var) => var.parse()?,
                None => false,
            }
        };
        if disable {
            return Ok(());
        }

        if let Some(bin_path) = env::var(format!("PANICHANDLER_TESTS_BIN_{}", $name)).ok() {
            Command::new(bin_path)
        } else {
            Command::cargo_bin($name)?
        }
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
            predicates::str::starts_with("PANIC: I paniced! Everything's awful! 1234\n").and(
                predicates::str::is_match(r#"(limiting \d* frames to 1000)"#)?,
            ),
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
