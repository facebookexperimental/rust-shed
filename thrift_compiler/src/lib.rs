/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

//! This crate is a wrapper around [fbthrift](https://github.com/facebook/fbthrift)'s compiler.
//! Its main usage is withing
//! [Cargo's build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
//! where it might be invoked to generate rust code from thrift files.

use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{create_dir_all, read_to_string, write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, ensure, Context, Result};

/// Builder for thrift compilare wrapper
pub struct Config {
    thrift_bin: OsString,
    out_dir: PathBuf,
    base_path: Option<PathBuf>,
    crate_map: Option<PathBuf>,
    options: Option<String>,
}

impl Config {
    /// Return a new configuration with the required parameters set
    pub fn new(thrift_bin: OsString, out_dir: PathBuf) -> Self {
        Self {
            thrift_bin,
            out_dir,
            base_path: None,
            crate_map: None,
            options: None,
        }
    }

    /// Return a new configuration with parameters computed based on environment
    /// variables set by Cargo's build scrip (OUT_DIR mostly). It requires that
    /// either "thrift1" is an executable callable by Command (e.g. on unix it
    /// means that it is in the PATH) or that the THRIFT environment variable
    /// points to a thrift executable
    pub fn from_env() -> Result<Self> {
        let thrift_bin = env::var_os("THRIFT").unwrap_or_else(|| OsStr::new("thrift1").to_owned());
        let out_dir = PathBuf::from(
            env::var("OUT_DIR")
                .with_context(|| anyhow!("The OUT_DIR environment variable must be set"))?,
        );

        let crate_map = out_dir.join("cratemap");
        let mut conf = Self::new(thrift_bin, out_dir);

        if crate_map.is_file() {
            conf.crate_map(crate_map);
        }

        Ok(conf)
    }

    /// Set the base path which is used by the compiler to find thrift files
    /// included by input thrift files
    pub fn base_path(&mut self, value: impl Into<PathBuf>) -> &mut Self {
        self.base_path = Some(value.into());
        self
    }

    /// Set the path to file with crate map definition which is used by the
    /// compiler to infer crate names that will be used in the generated code.
    /// Please refer to code in
    /// fbthrift/thrift/compiler/generate/t_mstch_rust_generator.cc
    /// for the scheme of crate map.
    pub fn crate_map(&mut self, value: impl Into<PathBuf>) -> &mut Self {
        self.crate_map = Some(value.into());
        self
    }

    /// Set the options to be passed to `mstch_rust` code generation. Example
    /// options are `serde`.
    pub fn options(&mut self, value: impl Into<String>) -> &mut Self {
        self.options = Some(value.into());
        self
    }

    /// Run the compiler on the input files. As a result a `lib.rs` file will
    /// be generated inside the output dir.
    pub fn run(&self, input_files: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<()> {
        let input = name_and_path_from_input(input_files)?;
        create_dir_all(&self.out_dir)?;

        if input.len() == 1 {
            self.run_compiler(&self.out_dir, input.into_iter().next().unwrap().1)?;
        } else {
            let partial_dir = self.out_dir.join("partial");
            create_dir_all(&partial_dir)?;

            write(
                self.out_dir.join("lib.rs"),
                input
                    .into_iter()
                    .map(|(name, file)| {
                        let out = partial_dir.join(&name);
                        create_dir_all(&out)?;
                        Ok(format!(
                            "pub mod {} {{\n{}\n}}\n",
                            name.to_string_lossy(),
                            self.run_compiler(out, file)?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?
                    .join("\n"),
            )?;
        }

        Ok(())
    }

    fn run_compiler(&self, out: impl AsRef<Path>, input: impl AsRef<Path>) -> Result<String> {
        let mut cmd = Command::new(&self.thrift_bin);

        let args = {
            let mut args = Vec::new();
            if let Some(crate_map) = &self.crate_map {
                args.push(format!("cratemap={}", crate_map.display()))
            }
            if let Some(base_path) = &self.base_path {
                args.push(format!("include_prefix={}", base_path.display()));
                cmd.arg("-I");
                cmd.arg(base_path);
            }
            if let Some(options) = &self.options {
                args.push(options.to_owned());
            }
            if args.is_empty() {
                "".to_owned()
            } else {
                format!(":{}", args.join(","))
            }
        };

        cmd.arg("--gen")
            .arg(format!("mstch_rust{}", args))
            .arg("--out")
            .arg(out.as_ref())
            .arg(input.as_ref());

        let output = cmd.output().with_context(|| {
            format!(
                "Failed to run thrift compiler. Is '{}' executable?",
                self.thrift_bin.to_string_lossy()
            )
        })?;
        ensure!(
            output.status.success(),
            format!(
                "Command '{:#?}' failed! Stdout:\n{}\nStderr:\n{}",
                cmd,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            )
        );

        let out_file = out.as_ref().join("lib.rs");
        ensure!(
            out_file.is_file(),
            format!(
                "Thrift has successfully run, but the resulting '{}' file is missing, command: '{:#?}'",
                out_file.display(),
                cmd,
            )
        );

        read_to_string(&out_file)
            .with_context(|| format!("Failed to read content of file '{}'", out_file.display()))
    }
}

fn name_and_path_from_input<T: AsRef<Path>>(
    input_files: impl IntoIterator<Item = T>,
) -> Result<Vec<(OsString, T)>> {
    input_files
        .into_iter()
        .map(|file| {
            Ok((
                file.as_ref()
                    .file_stem()
                    .ok_or_else(|| {
                        anyhow!(
                            "Failed to get file_stem from path {}",
                            file.as_ref().display()
                        )
                    })?
                    .to_owned(),
                file,
            ))
        })
        .collect::<Result<Vec<(OsString, _)>>>()
}
