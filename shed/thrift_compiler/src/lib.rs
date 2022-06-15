/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

//! This crate is a wrapper around [fbthrift](https://github.com/facebook/fbthrift)'s compiler.
//! Its main usage is withing
//! [Cargo's build script](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
//! where it might be invoked to generate rust code from thrift files.

use std::borrow::Cow;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{copy, create_dir_all, read_to_string, rename, write};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, ensure, Context, Result};
use which::which;

/// Builder for thrift compilare wrapper
pub struct Config {
    thrift_bin: Option<OsString>,
    out_dir: PathBuf,
    base_path: Option<PathBuf>,
    crate_map: Option<PathBuf>,
    options: Option<String>,
    include_srcs: Vec<String>,
}

impl Config {
    /// Return a new configuration with the required parameters set
    pub fn new(thrift_bin: Option<OsString>, out_dir: PathBuf) -> Self {
        Self {
            thrift_bin,
            out_dir,
            base_path: None,
            crate_map: None,
            options: None,
            include_srcs: vec![],
        }
    }

    /// Return a new configuration with parameters computed based on environment variables set by
    /// Cargo's build scrip (OUT_DIR mostly). If THRIFT is in the environment, that will be used as
    /// the Thrift binary. Otherwise, it will be detected in run_compiler.
    pub fn from_env() -> Result<Self> {
        println!("cargo:rerun-if-env-changed=THRIFT");

        let thrift_bin = env::var_os("THRIFT");
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

    /// Set the base path which is used by the compiler to find thrift files included by input
    /// thrift files. This is also used to find the compiler.
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

    /// Set extra srcs to be available in the generated crate.
    pub fn include_srcs(&mut self, value: Vec<String>) -> &mut Self {
        self.include_srcs = value;
        self
    }

    /// Run the compiler on the input files. As a result a `lib.rs` file will
    /// be generated inside the output dir.
    pub fn run(&self, input_files: impl IntoIterator<Item = impl AsRef<Path>>) -> Result<()> {
        let thrift_bin = self.resolve_thrift_bin()?;

        let input = name_and_path_from_input(input_files)?;
        create_dir_all(&self.out_dir)?;

        for input in &input {
            println!(
                "cargo:rerun-if-changed={}",
                input.1.as_ref().to_string_lossy()
            );
        }

        for include_src in &self.include_srcs {
            println!("cargo:rerun-if-changed={}", include_src);
            let from = PathBuf::from(include_src);
            let mut to = self.out_dir.clone();
            to.push(&from);
            copy(&from, &to)?;
        }

        if input.len() == 1 {
            self.run_compiler(
                thrift_bin.as_os_str(),
                &self.out_dir,
                input.into_iter().next().unwrap().1,
            )?;
        } else {
            let partial_dir = self.out_dir.join("partial");
            create_dir_all(&partial_dir)?;

            for (name, file) in &input {
                let out = partial_dir.join(&name);
                create_dir_all(&out)?;
                self.run_compiler(thrift_bin.as_os_str(), &out, file)?;
                rename(out.join("lib.rs"), out.join("mod.rs"))?;
            }

            let partial_lib_modules = input
                .iter()
                .map(|(name, _file)| format!("pub mod {};\n", name.to_string_lossy()))
                .collect::<Vec<_>>()
                .join("\n");
            write(partial_dir.join("mod.rs"), partial_lib_modules)?;

            let lib_modules = input
                .iter()
                .map(|(name, _file)| format!("pub use partial::{};\n", name.to_string_lossy()))
                .collect::<Vec<_>>()
                .join("\n");
            write(
                self.out_dir.join("lib.rs"),
                format!("pub mod partial;\n\n{}", lib_modules),
            )?;
        }

        Ok(())
    }

    fn resolve_thrift_bin(&self) -> Result<Cow<'_, OsString>> {
        // Get raw location
        let mut thrift_bin = if let Some(bin) = self.thrift_bin.as_ref() {
            Cow::Borrowed(bin)
        } else {
            Cow::Owned(self.infer_thrift_binary())
        };
        // Resolve based on PATH if needed
        let thrift_bin_path: &Path = thrift_bin.as_ref().as_ref();
        if thrift_bin_path.components().count() == 1 {
            println!("cargo:rerun-if-env-changed=PATH");
            let new_path = which(thrift_bin.as_ref()).with_context(|| {
                format!(
                    "Failed to resolve thrift binary `{}` to an absolute path",
                    thrift_bin.to_string_lossy()
                )
            })?;
            thrift_bin = Cow::Owned(new_path.into_os_string())
        }
        println!("cargo:rerun-if-changed={}", thrift_bin.to_string_lossy());
        Ok(thrift_bin)
    }

    fn infer_thrift_binary(&self) -> OsString {
        if let Some(base) = self.base_path.as_ref() {
            let mut candidate = base.clone();
            candidate.push("thrift/facebook/rpm/thrift1");
            #[cfg(windows)]
            candidate.set_extension("exe");
            if Path::new(&candidate).exists() {
                return candidate.into_os_string();
            }
        }

        "thrift1".into()
    }

    fn run_compiler(
        &self,
        thrift_bin: &OsStr,
        out: impl AsRef<Path>,
        input: impl AsRef<Path>,
    ) -> Result<String> {
        let mut cmd = Command::new(thrift_bin);

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
            if !self.include_srcs.is_empty() {
                args.push(format!("include_srcs={}", self.include_srcs.join(":")));
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
                thrift_bin.to_string_lossy()
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
