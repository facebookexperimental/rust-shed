/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

mod lrtd_tests_current {
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::Duration;

    use tokio_detectors::detectors::LongRunningTaskDetector;

    async fn run_blocking_stuff() {
        println!("slow start");
        thread::sleep(Duration::from_secs(1));
        println!("slow done");
    }

    #[test]
    fn test_blocking_detection_current() {
        let (lrtd, mut builder) = LongRunningTaskDetector::new_current_threaded(
            Duration::from_millis(10),
            Duration::from_millis(100),
        );
        let runtime = builder.enable_all().build().unwrap();
        let arc_runtime = Arc::new(runtime);
        let arc_runtime2 = arc_runtime.clone();
        lrtd.start(arc_runtime);
        arc_runtime2.block_on(async {
            run_blocking_stuff().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            println!("Done");
        });
    }

    #[test]
    fn test_blocking_detection_lambda() {
        let (lrtd, mut builder) = LongRunningTaskDetector::new_current_threaded(
            Duration::from_millis(10),
            Duration::from_millis(100),
        );
        let runtime = builder.enable_all().build().unwrap();
        let arc_runtime = Arc::new(runtime);
        let arc_runtime2 = arc_runtime.clone();
        let my_atomic_bool = Arc::new(AtomicBool::new(false));
        let my_atomic_bool2 = my_atomic_bool.clone();
        lrtd.start_with_custom_action(
            arc_runtime,
            Arc::new(move |workers: &_| {
                eprintln!("Blocking: {:?}", workers);
                my_atomic_bool.store(true, Ordering::SeqCst);
            }),
        );
        arc_runtime2.block_on(async {
            run_blocking_stuff().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            println!("Done");
        });
        assert!(my_atomic_bool2.load(Ordering::SeqCst));
    }
}

mod lrtd_tests_multi {
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use tokio_detectors::detectors::LongRunningTaskDetector;

    async fn run_blocking_stuff() {
        println!("slow start");
        thread::sleep(Duration::from_secs(1));
        println!("slow done");
    }

    #[test]
    fn test_blocking_detection_multi() {
        let (lrtd, mut builder) = LongRunningTaskDetector::new_multi_threaded(
            Duration::from_millis(10),
            Duration::from_millis(100),
        );
        let runtime = builder.worker_threads(2).enable_all().build().unwrap();
        let arc_runtime = Arc::new(runtime);
        let arc_runtime2 = arc_runtime.clone();
        lrtd.start(arc_runtime);
        arc_runtime2.spawn(run_blocking_stuff());
        arc_runtime2.spawn(run_blocking_stuff());
        arc_runtime2.block_on(async {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            println!("Done");
        });
    }

    #[test]
    fn test_blocking_detection_stop_unstarted() {
        let (_lrtd, _builder) = LongRunningTaskDetector::new_multi_threaded(
            Duration::from_millis(10),
            Duration::from_millis(100),
        );
    }
}

#[cfg(all(unix))]
mod unix_lrtd_tests {

    use std::backtrace::Backtrace;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::Duration;
    use std::time::Instant;

    use tokio_detectors::detectors::BlockingActionHandler;
    use tokio_detectors::detectors::LongRunningTaskDetector;
    use tokio_detectors::detectors::ThreadInfo;

    async fn run_blocking_stuff() {
        println!("slow start");
        thread::sleep(Duration::from_secs(1));
        println!("slow done");
    }

    fn get_thread_id() -> libc::pthread_t {
        unsafe { libc::pthread_self() }
    }

    static SIGNAL_COUNTER: AtomicUsize = AtomicUsize::new(0);

    static THREAD_DUMPS: Mutex<Option<HashMap<libc::pthread_t, String>>> = Mutex::new(None);

    extern "C" fn signal_handler(_: i32) {
        // not signal safe, this needs to be rewritten to avoid mem allocations and use a pre-allocated buffer.
        let backtrace = Backtrace::force_capture();
        let name = thread::current()
            .name()
            .map(|n| format!(" for thread \"{}\"", n))
            .unwrap_or_else(|| "".to_owned());
        let tid = get_thread_id();
        let detail = format!("Stack trace{}:{}\n{}", name, tid, backtrace);
        let mut omap = THREAD_DUMPS.lock().unwrap();
        let map = omap.as_mut().unwrap();
        (*map).insert(tid, detail);
        SIGNAL_COUNTER.fetch_sub(1, Ordering::SeqCst);
    }

    fn install_thread_stack_stace_handler(signal: libc::c_int) {
        unsafe {
            libc::signal(signal, signal_handler as libc::sighandler_t);
        }
    }

    static GTI_MUTEX: Mutex<()> = Mutex::new(());

    /// A naive stack trace capture implementation for threads for DEMO/TEST only purposes.
    fn get_thread_info(
        signal: libc::c_int,
        targets: &[ThreadInfo],
    ) -> HashMap<libc::pthread_t, String> {
        let _lock = GTI_MUTEX.lock();
        {
            let mut omap = THREAD_DUMPS.lock().unwrap();
            *omap = Some(HashMap::new());
            SIGNAL_COUNTER.store(targets.len(), Ordering::SeqCst);
        }
        for thread_info in targets {
            let result = unsafe { libc::pthread_kill(*thread_info.pthread_id(), signal) };
            if result != 0 {
                eprintln!("Error sending signal: {:?}", result);
            }
        }
        let time_limit = Duration::from_secs(1);
        let start_time = Instant::now();
        loop {
            let signal_count = SIGNAL_COUNTER.load(Ordering::SeqCst);
            if signal_count == 0 {
                break;
            }
            if Instant::now() - start_time >= time_limit {
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(10));
        }
        {
            let omap = THREAD_DUMPS.lock().unwrap();
            omap.clone().unwrap()
        }
    }

    struct DetailedCaptureBlockingActionHandler {
        inner: Mutex<Option<HashMap<libc::pthread_t, String>>>,
    }

    impl DetailedCaptureBlockingActionHandler {
        fn new() -> Self {
            DetailedCaptureBlockingActionHandler {
                inner: Mutex::new(None),
            }
        }

        fn contains_symbol(&self, symbol_name: &str) -> bool {
            // Iterate over the frames in the backtrace
            let omap = self.inner.lock().unwrap();
            match omap.as_ref() {
                Some(map) => {
                    if map.is_empty() {
                        false
                    } else {
                        let bt_str = map.values().next().unwrap();
                        bt_str.contains(symbol_name)
                    }
                }
                None => false,
            }
        }
    }

    impl BlockingActionHandler for DetailedCaptureBlockingActionHandler {
        fn blocking_detected(&self, workers: &[ThreadInfo]) {
            let mut map = self.inner.lock().unwrap();
            let tinfo = get_thread_info(libc::SIGUSR1, workers);
            eprintln!("Blocking detected with details: {:?}", tinfo);
            *map = Some(tinfo);
        }
    }

    #[test]
    fn test_blocking_detection_multi_capture_stack_traces() {
        install_thread_stack_stace_handler(libc::SIGUSR1);
        let (lrtd, mut builder) = LongRunningTaskDetector::new_multi_threaded(
            Duration::from_millis(10),
            Duration::from_millis(100),
        );
        let runtime = builder.worker_threads(2).enable_all().build().unwrap();
        let arc_runtime = Arc::new(runtime);
        let arc_runtime2 = arc_runtime.clone();
        let blocking_action = Arc::new(DetailedCaptureBlockingActionHandler::new());
        let to_assert_blocking = blocking_action.clone();
        lrtd.start_with_custom_action(arc_runtime, blocking_action);
        arc_runtime2.spawn(run_blocking_stuff());
        arc_runtime2.spawn(run_blocking_stuff());
        arc_runtime2.block_on(async {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            println!("Done");
        });
        assert!(to_assert_blocking.contains_symbol("std::thread::sleep"));
        lrtd.stop()
    }
}
