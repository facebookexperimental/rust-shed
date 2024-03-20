## Motivation

The motivation for this pull request is to provide a solution for the problem
described at: https://github.com/tokio-rs/console/issues/150

Blocking calls in the async workers lead to scalability issues, being able to
detect them so that code can be fixed is very important for the scalability of
an async-io service.

Lrtd running in production will detect blocked runtime workers and report them.
This allows to emit thread stack trace details that will enable owners to
quickly identify the issue and push out a fix, further enhancing the scalability
and responsiveness of their services. The blocking event counts can be used to
test the health of the canary tier, etc...

## Solution

LongRunningTaskDetector uses a sampling black box approach to detect when
processing in the tokio runtime in blocked for more than a configurable amount.
The approach is simple and reliable, and has very low/configurable overhead, as
such is is ideal for continuous running in production environments.

The LongRunningTaskDetector can be started like:

```
    use std::sync::Arc;
    use tokio_metrics::detectors::LongRunningTaskDetector;

    let (lrtd, mut builder) = LongRunningTaskDetector::new_multi_threaded(
      std::time::Duration::from_millis(10),
      std::time::Duration::from_millis(100)
    );
    let runtime = builder.worker_threads(2).enable_all().build().unwrap();
    let runtime_ref = Arc::new(runtime);
    let lrtd_runtime_ref = arc_runtime.clone();
    lrtd.start(lrtd_runtime_ref);
    runtime_ref.block_on(async {
     print!("my async code")
    });
```

When blocking is detected the stack traces of the runtime worker threads can be
dumped (action is plugable), to allow us to identify what is happening in the
worker threads.

On detection, details can contain precise info about the blocking culprit and
will look like:

```
Detected worker blocking, signaling SIGUSR1 worker threads: [123145546047488]
Stack trace for thread "test_blocking_detection_current":123145546047488
   0: std::backtrace_rs::backtrace::libunwind::trace
             at /rustc/a28077b28a02b92985b3a3faecf92813155f1ea1/library/std/src/../../backtrace/src/backtrace/libunwind.rs:93:5
   1: std::backtrace_rs::backtrace::trace_unsynchronized
             at /rustc/a28077b28a02b92985b3a3faecf92813155f1ea1/library/std/src/../../backtrace/src/backtrace/mod.rs:66:5
   2: std::backtrace::Backtrace::create
             at /rustc/a28077b28a02b92985b3a3faecf92813155f1ea1/library/std/src/backtrace.rs:331:13
   3: std::backtrace::Backtrace::force_capture
             at /rustc/a28077b28a02b92985b3a3faecf92813155f1ea1/library/std/src/backtrace.rs:313:9
   4: tokio_util::lrtd::lrtd::signal_handler
             at ./src/lrtd/lrtd.rs:221:21
   5: __sigtramp
   6: ___semwait_signal
   7: <unknown>
   8: std::sys::unix::thread::Thread::sleep
             at /rustc/a28077b28a02b92985b3a3faecf92813155f1ea1/library/std/src/sys/unix/thread.rs:241:20
   9: std::thread::sleep
             at /rustc/a28077b28a02b92985b3a3faecf92813155f1ea1/library/std/src/thread/mod.rs:872:5
  10: lrtd::run_blocking_stuff::{{closure}}
             at ./tests/lrtd.rs:12:5
  11: lrtd::test_blocking_detection_current::{{closure}}
             at ./tests/lrtd.rs:57:30
  12: <core::pin::Pin<P> as core::future::future::Future>::poll
             at /rustc/a28077b28a02b92985b3a3faecf92813155f1ea1/library/core/src/future/future.rs:125:9
  13: tokio::runtime::scheduler::current_thread::CoreGuard::block_on::{{closure}}::{{closure}}::{{closure}}
....
```
