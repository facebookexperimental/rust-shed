/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

#![doc = include_str!("../README.md")]

mod mocks;
mod tokio_impl;

use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::Path;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;

pub use crate::mocks::*;
pub use crate::tokio_impl::*;

/// Provides an interface for accepting a [`Command`] and returning a [`Child`].
#[async_trait]
pub trait CommandSpawner: Clone + Sync + Send + 'static {
    type Command: Command;
    type Child: Child;

    /// Executes the command as a child process, returning a [`Child`].
    ///
    /// See also: [`tokio::process::Command::spawn`].
    fn spawn(&self, command: &mut Self::Command) -> io::Result<Self::Child>;

    /// A convenience wrapper over [`Child::wait_with_output`] that sets `stdout`/`stderr` to
    /// [`Stdio::piped`] outputs before calling [`Self::spawn`].
    ///
    /// See also: [`tokio::process::Command::output`] and [`Child::wait_with_output`].
    async fn output(
        &self,
        command: &mut Self::Command,
    ) -> io::Result<<Self::Child as Child>::Output> {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        self.spawn(command)?.wait_with_output().await
    }
}

/// Provides a mockable trait wrapper around [`tokio::process::Command`].
pub trait Command: Debug + Sync + Send {
    /// Constructs a new [`Command`] for launching the program at path `program`.
    ///
    /// See also: [`tokio::process::Command::new`].
    fn new(program: impl AsRef<OsStr>) -> Self;
    fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self;
    fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> &mut Self;

    /// Sets the current working directory.
    ///
    /// Tokio recommends using absolute paths to avoid possible platform-specific ambiguity.
    fn current_dir(&mut self, dir: impl AsRef<Path>) -> &mut Self;

    /// Inserts or updates an environment variable.
    fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self;
    /// Clears all environment variables.
    fn env_clear(&mut self) -> &mut Self;
    /// Clears a specific environment variable.
    fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self;
    /// Adds or updates multiple environment variables.
    fn envs<K: AsRef<OsStr>, V: AsRef<OsStr>>(
        &mut self,
        vars: impl IntoIterator<Item = (K, V)>,
    ) -> &mut Self;

    #[cfg(unix)]
    fn uid(&mut self, id: u32) -> &mut Self;
    #[cfg(unix)]
    fn gid(&mut self, id: u32) -> &mut Self;

    fn stdin(&mut self, cfg: impl Into<Stdio>) -> &mut Self;
    fn stdout(&mut self, cfg: impl Into<Stdio>) -> &mut Self;
    fn stderr(&mut self, cfg: impl Into<Stdio>) -> &mut Self;

    /// If `true`, spawned child processes will be killed when the `Child` object is dropped.
    ///
    /// By default, this is `false`.
    ///
    /// This is done on a best-effort basis. [`Child::wait`], [`Child::kill`], or
    /// [`Child::terminate`] should typically be preferred.
    ///
    /// See also: [`tokio::process::Command::kill_on_drop`].
    fn kill_on_drop(&mut self, kill_on_drop: bool) -> &mut Self;
}

/// Provides a mockable trait wrapper around [`tokio::process::Child`].
#[async_trait]
pub trait Child: Send {
    type Stdin: AsyncWrite + Send + Unpin + 'static;
    type Stdout: AsyncRead + Send + Unpin + 'static;
    type Stderr: AsyncRead + Send + Unpin + 'static;
    type ExitStatus: ExitStatus;
    type Output: Output<ExitStatus = Self::ExitStatus>;

    fn stdin(&mut self) -> &mut Option<Self::Stdin>;
    fn stdout(&mut self) -> &mut Option<Self::Stdout>;
    fn stderr(&mut self) -> &mut Option<Self::Stderr>;

    /// Unix only: Sends SIGTERM to the child process and waits for it to exit.
    ///
    /// Warning: As the child can ignore `SIGTERM`, this can hang, and should usually be combined
    /// with a timeout.
    #[cfg(unix)]
    async fn terminate(&mut self) -> io::Result<()>;

    /// Forces the child to exit. On Unix-like platforms, this uses `SIGKILL`.
    async fn kill(&mut self) -> io::Result<()>;

    /// Checks if the child is still running. Returns immediately.
    ///
    /// - Returns with `Ok(None)` if the process is still running.
    /// - Returns with `Ok(Some(...))` if the process has exited.
    /// - Returns with `Ok(Some(...))` if the process has exited.
    fn try_wait(&mut self) -> io::Result<Option<Self::ExitStatus>>;

    /// Waits until the process exits, then returns.
    async fn wait(&mut self) -> io::Result<Self::ExitStatus>;

    /// Resolves an [`Output`] object with the exit status and entire stdout/stderr bytes.
    ///
    /// Make sure to call [`Command::stdout`] and [`Command::stderr`] with [`Stdio::piped()`] when
    /// configuring the command, before calling this.
    ///
    /// [`CommandSpawner::output`] exists as a convenience wrapper over this API.
    ///
    /// See also: [`Command::output`] and [`tokio::process::Child::wait_with_output`].
    async fn wait_with_output(self) -> io::Result<Self::Output>;
}

/// Provides a mockable trait wrapper around [`std::process::ExitStatus`]. This is needed because
/// normal [`ExitStatus`] type cannot be constructed (or therefore used in a mock).
pub trait ExitStatus: Sync + Send {
    fn success(&self) -> bool;
    fn code(&self) -> Option<i32>;
}

/// Provides a wrapper around [`std::process::Output`]. This is needed because we must use our
/// mockable [`ExitStatus`] trait.
pub trait Output: Sync + Send {
    type ExitStatus: ExitStatus;

    fn status(&self) -> &Self::ExitStatus;
    fn stdout(&self) -> &Vec<u8>;
    fn stderr(&self) -> &Vec<u8>;
}
