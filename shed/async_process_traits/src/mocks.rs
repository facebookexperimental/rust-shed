/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::collections::HashMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::future;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

use async_trait::async_trait;
use tokio::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio::sync::Mutex as TokioMutex;
use tokio::sync::watch;

use crate::Child;
use crate::Command;
use crate::CommandSpawner;
use crate::ExitStatus;
use crate::Output;

#[derive(Clone)]
pub struct MockCommandSpawner(
    /// A callback that gets called by [`MockCommandSpawner::spawn`].
    Arc<StdMutex<dyn FnMut(&mut MockCommand) -> io::Result<MockChild> + Sync + Send + 'static>>,
);

impl MockCommandSpawner {
    pub fn new() -> Self {
        Self::with_callback(|_cmd| Ok(MockChild::new(MockChildHandle::new())))
    }

    pub fn with_handle(handle: MockChildHandle) -> Self {
        Self::with_callback(move |_cmd| Ok(MockChild::new(handle.clone())))
    }

    pub fn with_callback(
        callback: impl FnMut(&mut MockCommand) -> io::Result<MockChild> + Sync + Send + 'static,
    ) -> Self {
        Self(Arc::new(StdMutex::new(callback)))
    }
}

impl CommandSpawner for MockCommandSpawner {
    type Command = MockCommand;
    type Child = MockChild;

    fn spawn(&self, command: &mut Self::Command) -> io::Result<Self::Child> {
        (self.0.lock().unwrap())(command)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct MockCommand {
    pub program: OsString,
    pub args: Vec<OsString>,
    pub env: HashMap<OsString, OsString>,
    pub current_dir: Option<PathBuf>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub kill_on_drop: bool,
    // don't store stdin/stdout/stderr Stdio objects, as they're opaque, and therefore not useful
    // for comparing against.
}

impl Command for MockCommand {
    fn new(program: impl AsRef<OsStr>) -> Self {
        Self {
            program: OsString::from(&program),
            args: Vec::new(),
            env: HashMap::new(),
            current_dir: None,
            uid: None,
            gid: None,
            kill_on_drop: false,
        }
    }
    fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.args.push(OsString::from(&arg));
        self
    }
    fn args(&mut self, args: impl IntoIterator<Item = impl AsRef<OsStr>>) -> &mut Self {
        self.args
            .extend(args.into_iter().map(|a| OsString::from(&a)));
        self
    }
    fn current_dir(&mut self, dir: impl AsRef<Path>) -> &mut Self {
        self.current_dir = Some(dir.as_ref().into());
        self
    }

    fn env(&mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> &mut Self {
        self.env
            .insert(key.as_ref().to_owned(), value.as_ref().to_owned());
        self
    }
    fn env_clear(&mut self) -> &mut Self {
        self.env.clear();
        self
    }
    fn env_remove(&mut self, key: impl AsRef<OsStr>) -> &mut Self {
        self.env.remove(key.as_ref());
        self
    }
    fn envs<K: AsRef<OsStr>, V: AsRef<OsStr>>(
        &mut self,
        vars: impl IntoIterator<Item = (K, V)>,
    ) -> &mut Self {
        for (k, v) in vars {
            self.env(k, v);
        }
        self
    }

    #[cfg(unix)]
    fn uid(&mut self, id: u32) -> &mut Self {
        self.uid = Some(id);
        self
    }
    #[cfg(unix)]
    fn gid(&mut self, id: u32) -> &mut Self {
        self.gid = Some(id);
        self
    }
    fn stdin(&mut self, _cfg: impl Into<Stdio>) -> &mut Self {
        self
    }
    fn stdout(&mut self, _cfg: impl Into<Stdio>) -> &mut Self {
        self
    }
    fn stderr(&mut self, _cfg: impl Into<Stdio>) -> &mut Self {
        self
    }
    fn kill_on_drop(&mut self, kill_on_drop: bool) -> &mut Self {
        self.kill_on_drop = kill_on_drop;
        self
    }
}

type BoxAsyncRead = Box<dyn AsyncRead + Send + Unpin>;
type BoxAsyncWrite = Box<dyn AsyncWrite + Send + Unpin>;

pub struct MockChild {
    stdin: Option<BoxAsyncWrite>,
    stdout: Option<BoxAsyncRead>,
    stderr: Option<BoxAsyncRead>,
    status: watch::Receiver<Result<Option<MockExitStatus>, io::ErrorKind>>,
    handle: MockChildHandle,
}

impl MockChild {
    pub fn new(handle: MockChildHandle) -> Self {
        Self::with_stdio(
            handle,
            /* stdin */ Some(Box::new(io::sink())),
            /* stdout */ Some(Box::new(io::empty())),
            /* stderr */ Some(Box::new(io::empty())),
        )
    }

    pub fn with_stdio(
        handle: MockChildHandle,
        stdin: Option<impl AsyncWrite + Send + Unpin + 'static>,
        stdout: Option<impl AsyncRead + Send + Unpin + 'static>,
        stderr: Option<impl AsyncRead + Send + Unpin + 'static>,
    ) -> Self {
        let status = handle.inner.lock().unwrap().status.subscribe();
        Self {
            stdin: stdin.map(|el| Box::new(el) as BoxAsyncWrite),
            stdout: stdout.map(|el| Box::new(el) as BoxAsyncRead),
            stderr: stderr.map(|el| Box::new(el) as BoxAsyncRead),
            status,
            handle,
        }
    }
}

#[async_trait]
impl Child for MockChild {
    type Stdin = BoxAsyncWrite;
    type Stdout = BoxAsyncRead;
    type Stderr = BoxAsyncRead;
    type ExitStatus = MockExitStatus;
    type Output = MockOutput;

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
        if self.try_wait()?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid argument: can't terminate an exited process",
            ));
        }
        let mock_terminate = self.handle.inner.lock().unwrap().mock_terminate.clone();
        mock_terminate().await?;
        self.handle.inner.lock().unwrap().terminated = true;
        self.handle
            .set_status(Ok(Some(Self::ExitStatus::new(None))));
        Ok(())
    }

    async fn kill(&mut self) -> io::Result<()> {
        if self.try_wait()?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid argument: can't kill an exited process",
            ));
        }
        let mock_kill = self.handle.inner.lock().unwrap().mock_kill.clone();
        mock_kill().await?;
        self.handle.inner.lock().unwrap().killed = true;
        self.handle
            .set_status(Ok(Some(Self::ExitStatus::new(None))));
        Ok(())
    }

    fn try_wait(&mut self) -> io::Result<Option<Self::ExitStatus>> {
        match &*self.status.borrow() {
            Ok(status) => Ok(status.clone()),
            Err(err_kind) => Err(io::Error::new(*err_kind, "mock error!")),
        }
    }

    async fn wait(&mut self) -> io::Result<Self::ExitStatus> {
        loop {
            match self.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => {} // continue
                Err(err) => return Err(err),
            }
            self.status
                .changed()
                .await
                .expect("status.changed() failed. Was MockChildHandle dropped?")
        }
    }

    async fn wait_with_output(mut self) -> io::Result<Self::Output> {
        async fn handle_pipe(maybe_pipe: Option<BoxAsyncRead>) -> io::Result<Vec<u8>> {
            let mut out = Vec::new();
            if let Some(mut pipe) = maybe_pipe {
                pipe.read_to_end(&mut out).await?;
            }
            Ok(out)
        }

        let (stdout_pipe, stderr_pipe) = (self.stdout().take(), self.stderr().take());
        let (status, stdout, stderr) = tokio::try_join!(
            self.wait(),
            handle_pipe(stdout_pipe),
            handle_pipe(stderr_pipe),
        )?;
        Ok(MockOutput {
            status,
            stdout,
            stderr,
        })
    }
}

/// Allows inspection of the child's state or modification of the child's exit status without
/// needing ownership of the [`Child`]. Handles are internally stored using [`Arc`] and can be
/// cloned, allowing for both the test and the [`MockChild`] to "own" the same handle.
///
/// ```
/// use async_process_traits::Child;
/// use async_process_traits::MockChild;
/// use async_process_traits::MockChildHandle;
/// use tokio::io;
///
/// struct TestedComponentUsesChild<Ch: Child> {
///     child: Ch,
///     // ...
/// }
///
/// impl<Ch: Child> TestedComponentUsesChild<Ch> {
///     fn new(child: Ch) -> Self {
///         Self { child }
///     }
///
///     async fn cleanup(&mut self) -> io::Result<()> {
///         self.child.kill().await
///     }
/// }
///
/// #[tokio::test]
/// async fn test_component() {
///     let handle = MockChildHandle::new();
///     // clone the handle when constructing MockChild
///     let child = MockChild::new(handle.clone());
///     // pass ownership of `child` to `TestedComponentUsesChild`
///     let tested_component = TestedComponentUsesChild::new(child);
///
///     // we've still got a handle, and can make assertions about the code
///     assert!(!handle.killed());
///     tested_component.cleanup().await.unwrap();
///     assert!(handle.killed());
/// }
/// ```
///
/// Tests may not have an easy way to get ownership of a spawned [`Child`], especially if the
/// executor uses an API like [`tokio::process::Child::wait`] that requires long-lived exclusive
/// access to the child.
#[derive(Clone)]
pub struct MockChildHandle {
    inner: Arc<StdMutex<MockChildHandleInner>>,
}

struct MockChildHandleInner {
    killed: bool,
    terminated: bool,
    status: watch::Sender<Result<Option<MockExitStatus>, io::ErrorKind>>,
    mock_terminate:
        Arc<dyn Fn() -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> + Sync + Send>,
    mock_kill: Arc<dyn Fn() -> Pin<Box<dyn Future<Output = io::Result<()>> + Send>> + Sync + Send>,
    // Dummy field to ensure that `set_status` never fails (there's always at least one receiver),
    // even if the MockChild hasn't been constructed yet. This allows callers to mock the exit code
    // at the top of a test, before the MockChild is actually spawned.
    _status_rx: watch::Receiver<Result<Option<MockExitStatus>, io::ErrorKind>>,
}

impl MockChildHandle {
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(Ok(None));
        Self {
            inner: Arc::new(StdMutex::new(MockChildHandleInner {
                killed: false,
                terminated: false,
                status: tx,
                mock_terminate: Arc::new(|| Box::pin(future::ready(Ok(())))),
                mock_kill: Arc::new(|| Box::pin(future::ready(Ok(())))),
                _status_rx: rx,
            })),
        }
    }

    pub fn set_status(&self, status: Result<Option<MockExitStatus>, io::ErrorKind>) {
        self.inner
            .lock()
            .unwrap()
            .status
            .send(status)
            .expect("the MockChildHandleInner::_status_rx object should ensure send does not fail")
    }

    pub fn killed(&self) -> bool {
        self.inner.lock().unwrap().killed
    }

    pub fn terminated(&self) -> bool {
        self.inner.lock().unwrap().terminated
    }

    pub fn mock_terminate<Fut>(&self, cb: impl FnMut() -> Fut + Send + 'static)
    where
        Fut: Future<Output = io::Result<()>> + Send + 'static,
    {
        // It's useful to accept a FnMut, but mock_terminate needs to be stored/called as an
        // `Arc<dyn Fn() -> ...>`, so throw in a mutex here. This should work fine as long as we're
        // not terminating/killing multiple children at once.
        let cb = Arc::new(TokioMutex::new(cb));
        self.inner.lock().unwrap().mock_terminate = Arc::new(move || {
            let cb = Arc::clone(&cb);
            Box::pin(async move {
                let mut cb = cb.lock().await;
                cb().await
            })
        });
    }

    pub fn mock_kill<Fut>(&self, cb: impl FnMut() -> Fut + Send + 'static)
    where
        Fut: Future<Output = io::Result<()>> + Send + 'static,
    {
        let cb = Arc::new(TokioMutex::new(cb));
        self.inner.lock().unwrap().mock_kill = Arc::new(move || {
            let cb = Arc::clone(&cb);
            Box::pin(async move {
                let mut cb = cb.lock().await;
                cb().await
            })
        });
    }
}

#[derive(Clone, Debug)]
pub struct MockExitStatus {
    code: Option<i32>,
}

impl MockExitStatus {
    pub fn new(code: Option<i32>) -> Self {
        Self { code }
    }
}

impl ExitStatus for MockExitStatus {
    fn success(&self) -> bool {
        self.code == Some(0)
    }
    fn code(&self) -> Option<i32> {
        self.code
    }
}

#[derive(Debug)]
pub struct MockOutput {
    status: MockExitStatus,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl Output for MockOutput {
    type ExitStatus = MockExitStatus;

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
