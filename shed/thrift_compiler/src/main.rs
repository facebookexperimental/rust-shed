/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::env;
use std::path::PathBuf;

use anyhow::Result;
use clap::clap_app;
use thrift_compiler::Config;
use thrift_compiler::GenContext;

fn main() -> Result<()> {
    let matches = clap_app!(thrift_compiler =>
        (about: "Calls thrift compiler to produce unified lib.rs from thrift files")
        (@arg thrift: -t --thrift +takes_value "Path or name in PATH of thrift compiler binary (default: thrift1)")
        (@arg out: -o --out +takes_value "Directory where the result will be saved (default: .)")
        (@arg use_env: -e --use-environment "Uses environment variables instead of command line arguments")
        (@arg input: +required +takes_value ... "Paths to .thrift files")
        (@arg gen_context: -g --context +takes_value "Generation context: 'lib' or 'types' (default:'lib')")
    ).get_matches();

    let out = matches
        .value_of_os("out")
        .map_or_else(env::current_dir, |x| Ok(PathBuf::from(x)))?;
    let input = matches.values_of_os("input").unwrap();
    let gen_context = GenContext::try_from(matches.value_of("gen_context").unwrap_or("lib"))?;
    let compiler = if matches.is_present("use_env") {
        Config::from_env(gen_context)?
    } else {
        Config::new(gen_context, None, out)?
    };
    compiler.run(input)
}
