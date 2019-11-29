/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use std::env;
use std::ffi::OsStr;
use std::path::PathBuf;

use anyhow::Result;
use clap::clap_app;

use thrift_compiler::Config;

fn main() -> Result<()> {
    let matches = clap_app!(thrift_compiler =>
        (about: "Calls thrift compiler to produce unified lib.rs from thrift files")
        (@arg thrift: -t --thrift +takes_value "Path or name in PATH of thrift compiler binary (default: thrift1)")
        (@arg out: -o --out +takes_value "Directory where the result will be saved (default: .)")
        (@arg use_env: -e --use-environment "Uses environment variables instead of command line arguments")
        (@arg input: +required +takes_value ... "Paths to .thrift files")
    ).get_matches();

    let thrift = matches
        .value_of_os("thrift")
        .unwrap_or_else(|| OsStr::new("thrift1"))
        .to_owned();
    let out = matches
        .value_of_os("out")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir()?);
    let input = matches.values_of_os("input").unwrap();

    let compiler = if matches.is_present("use_env") {
        Config::from_env()?
    } else {
        Config::new(thrift, out)
    };
    compiler.run(input)
}
