/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::ffi::OsStr;
use std::path::Path;
pub use std::process::ExitStatus as TokioExitStatus;
use std::process::Output as TokioOutput;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::io;
pub use tokio::process::Child as TokioChild;
use tokio::process::ChildStderr;
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
pub use tokio::process::Command as TokioCommand;

use crate::Child;
use crate::Command;
use crate::CommandSpawner;
use crate::ExitStatus;
use crate::Output;

#[derive(Clone)]
pub struct TokioCommandSpawner;

impl CommandSpawner for TokioCommandSpawner {
    type Command = TokioCommand;
    type Child = TokioChild;

    fn spawn(&self, command: &mut Self::Command) -> io::Result<Self::Child> {
        command.spawn()
    }
}

impl Command for TokioCommand {
    fn new(program: impl AsRef<OsStr>) -> Self {
        Self::new(program)
    }
    fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.arg(arg)
    }
    fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> &mut Self {
        self.args(args)
    }
    fn current_dir(&mut self, dir: impl AsRef<Path>) -> &mut Self {
        self.current_dir(dir)
    }
    fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.env(key, value)
    }
    fn env_clear(&mut self) -> &mut Self {
        self.env_clear()
    }
    fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.env_remove(key)
    }
    fn envs<K: AsRef<OsStr>, V: AsRef<OsStr>>(
        &mut self,
        vars: impl IntoIterator<Item = (K, V)>,
    ) -> &mut Self {
        self.envs(vars)
    }
    #[cfg(unix)]
    fn uid(&mut self, id: u32) -> &mut Self {
        self.uid(id)
    }
    #[cfg(unix)]
    fn gid(&mut self, id: u32) -> &mut Self {
        self.gid(id)
    }
    fn stdin(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.stdin(cfg)
    }
    fn stdout(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.stdout(cfg)
    }
    fn stderr(&mut self, cfg: impl Into<Stdio>) -> &mut Self {
        self.stderr(cfg)
    }
    fn kill_on_drop(&mut self, kill_on_drop: bool) -> &mut Self {
        self.kill_on_drop(kill_on_drop)
    }
}

#[async_trait]
impl Child for TokioChild {
    type Stdin = ChildStdin;
    type Stdout = ChildStdout;
    type Stderr = ChildStderr;
    type ExitStatus = TokioExitStatus;
    type Output = TokioOutput;

    fn stdin(&mut self) -> &mut Option<Self::Stdin> {
        &mut self.stdin
    }
    fn stdout(&mut self) -> &mut Option<Self::Stdout> {
        &mut self.stdout
    }
    fn stderr(&mut self) -> &mut Option<Self::Stderr> {
        &mut self.stderr
    }
    #[cfg(unix)]
    async fn terminate(&mut self) -> io::Result<()> {
        use nix::sys::signal::Signal;
        use nix::sys::signal::kill;
        use nix::unistd::Pid;

        if let Some(id) = self.id() {
            kill(
                Pid::from_raw(id.try_into().expect("linux pid should fit in 22 bits")),
                Signal::SIGTERM,
            )
            .map_err(io::Error::other)?;
            // wait should clean up any possible zombie processes
            self.wait().await?;
            Ok(())
        } else {
            // if id is `None` the process has already terminated
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid argument: can't terminate an exited process",
            ))
        }
    }
    async fn kill(&mut self) -> io::Result<()> {
        self.kill().await
    }
    fn try_wait(&mut self) -> io::Result<Option<Self::ExitStatus>> {
        self.try_wait()
    }
    async fn wait(&mut self) -> io::Result<Self::ExitStatus> {
        self.wait().await
    }
    async fn wait_with_output(self) -> io::Result<Self::Output> {
        self.wait_with_output().await
    }
}

impl ExitStatus for TokioExitStatus {
    fn success(&self) -> bool {
        self.success()
    }
    fn code(&self) -> Option<i32> {
        self.code()
    }
}

impl Output for TokioOutput {
    type ExitStatus = TokioExitStatus;

    fn status(&self) -> &Self::ExitStatus {
        &self.status
    }
    fn stdout(&self) -> &Vec<u8> {
        &self.stdout
    }
    fn stderr(&self) -> &Vec<u8> {
        &self.stderr
    }
}

#[cfg(all(unix, test))]
mod tests {
    use std::time::Duration;

    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    use tokio::io::AsyncBufReadExt;
    use tokio::io::BufReader;
    use tokio::time::sleep;

    use super::*;

    fn is_pid_alive(raw_pid: i32) -> bool {
        // the PID could be recycled (technically this is a race), but that's unlikely during the
        // short duration of the unit test

        // don't actually send a signal, just check the pid exists
        kill(Pid::from_raw(raw_pid), None).is_ok()
    }

    /// Wraps a TokioCommand in an opaque Command type, to ensure you can only call methods defined
    /// as part of the Command trait.
    fn as_impl_command(cmd: &mut TokioCommand) -> &mut (impl Command + use<>) {
        cmd
    }

    /// Make sure that terminate waits for the process to exit and doesn't leave us with zombie
    /// processes.
    #[tokio::test]
    async fn test_terminate_waits_and_reaps_child() {
        // have bash echo its PID and sleep for a long time
        let mut command = TokioCommand::new("/bin/bash");
        as_impl_command(&mut command)
            .env_remove("LD_PRELOAD")
            .args(["-c", "echo $$; sleep 100"])
            .stdout(Stdio::piped());
        let mut child = TokioCommandSpawner.spawn(&mut command).unwrap();

        // read the child's PID
        let mut stdout = BufReader::new(child.stdout().take().unwrap());
        let mut buf = String::new();
        stdout.read_line(&mut buf).await.unwrap();
        let pid: i32 = buf.trim_end().parse().unwrap();

        assert!(is_pid_alive(pid));

        child.terminate().await.unwrap();

        assert!(!is_pid_alive(pid));
    }

    #[tokio::test]
    async fn test_kill_on_drop() {
        // have bash echo its PID and sleep for a long time
        let mut command = TokioCommand::new("/bin/bash");
        as_impl_command(&mut command)
            .env_remove("LD_PRELOAD")
            .args(["-c", "echo $$; sleep 100"])
            .kill_on_drop(true)
            .stdout(Stdio::piped());
        let mut child = TokioCommandSpawner.spawn(&mut command).unwrap();

        // read the child's PID
        let mut stdout = BufReader::new(child.stdout().take().unwrap());
        let mut buf = String::new();
        stdout.read_line(&mut buf).await.unwrap();
        let pid: i32 = buf.trim_end().parse().unwrap();

        assert!(is_pid_alive(pid));

        // dropping `child` should cause it to be killed
        drop(child);

        // sleep for up to 5 seconds
        for _i in 0..5 {
            sleep(Duration::from_secs(1)).await;
            if !is_pid_alive(pid) {
                break;
            }
        }

        // the child should have been killed!
        assert!(!is_pid_alive(pid));
    }

    #[tokio::test]
    async fn test_output() {
        let mut command = TokioCommand::new("/bin/bash");
        as_impl_command(&mut command)
            .env_remove("LD_PRELOAD")
            .args(["-c", "echo stdout && echo stderr 1>&2; exit 123"]);
        let output = TokioCommandSpawner.output(&mut command).await.unwrap();
        assert_eq!(output.status.code(), Some(123));
        assert_eq!(&output.stdout, b"stdout\n");
        assert_eq!(&output.stderr, b"stderr\n");
    }

    #[tokio::test]
    async fn test_current_dir() {
        let mut command = TokioCommand::new("/bin/bash");
        as_impl_command(&mut command)
            .env_remove("LD_PRELOAD")
            .args(["-c", "pwd"])
            .current_dir("/etc");
        let output = TokioCommandSpawner.output(&mut command).await.unwrap();

        // only inspect that last 5 bytes...
        // sandcastle forces test processes to run inside a /private prefix directory
        assert_eq!(&output.stdout[output.stdout.len() - 5..], b"/etc\n");
    }

    #[tokio::test]
    async fn test_env() {
        let mut command = TokioCommand::new("/usr/bin/env");
        as_impl_command(&mut command)
            .env_clear()
            .envs([("A", "1"), ("B", "2"), ("C", "3"), ("D", "4")])
            .env_remove("B")
            .env("A", "0");
        let output = TokioCommandSpawner.output(&mut command).await.unwrap();

        // only inspect that last 5 bytes...
        // sandcastle forces test processes to run inside a /private prefix directory
        assert_eq!(&output.stdout, b"A=0\nC=3\nD=4\n");
    }
}
