/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::fmt::Arguments;
use std::pin::Pin;
use std::time::Duration;
use std::time::Instant;

use futures::stream::Stream;
use futures::task::Context;
use futures::task::Poll;
use maybe_owned::MaybeOwned;
use pin_project::pin_project;
use slog::Logger;
use slog::Record;

/// If the budget is exceeded, we will log a warning if the total overshoot is more than this multiplier.
const BUDGET_OVERSHOOT_MULTIPLIER: u32 = 3;

/// A stream that will yield control back to the caller if it runs for more than a given duration
/// without yielding (i.e. returning Poll::Pending).  The clock starts counting the first time the
/// stream is polled, and is reset every time the stream yields.
#[pin_project]
pub struct YieldPeriodically<'a, S> {
    #[pin]
    inner: S,
    /// Default budget.
    budget: Duration,
    /// Budget left for the current iteration.
    current_budget: Duration,
    /// Whether the next iteration must yield because the budget was exceeded.
    must_yield: bool,
    /// The code location where yield_periodically was called.
    location: slog::RecordLocation,
    /// Enable logging to the provided logger when the budget is exceeded by
    /// BUDGET_OVERSHOOT_MULTIPLIER times or more.
    logger: Option<MaybeOwned<'a, Logger>>,
}

impl<S> YieldPeriodically<'_, S> {
    /// Create a new [YieldPeriodically].
    pub fn new(inner: S, location: slog::RecordLocation, budget: Duration) -> Self {
        Self {
            inner,
            budget,
            current_budget: budget,
            must_yield: false,
            location,
            logger: None,
        }
    }

    /// Set the budget for this stream.
    pub fn with_budget(mut self, budget: Duration) -> Self {
        self.budget = budget;
        self.current_budget = budget;
        self
    }

    /// Enable debug logging.
    pub fn with_logger<'a, L>(self, logger: L) -> YieldPeriodically<'a, S>
    where
        L: Into<MaybeOwned<'a, Logger>>,
    {
        YieldPeriodically {
            logger: Some(logger.into()),
            ..self
        }
    }
}

impl<S: Stream> Stream for YieldPeriodically<'_, S> {
    type Item = <S as Stream>::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();

        if *this.must_yield {
            *this.must_yield = false;
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }

        let now = Instant::now();
        let res = this.inner.poll_next(cx);

        if res.is_pending() {
            *this.current_budget = *this.budget;
            return res;
        }

        let current_budget = *this.current_budget;
        let elapsed = now.elapsed();

        match this.current_budget.checked_sub(elapsed) {
            Some(new_budget) => *this.current_budget = new_budget,
            None => {
                if (elapsed - current_budget) > *this.budget * BUDGET_OVERSHOOT_MULTIPLIER {
                    maybe_log(
                        this.logger,
                        this.location,
                        &format_args!(
                            "yield_periodically(): budget overshot: current_budget={:?}, elapsed={:?}",
                            current_budget, elapsed,
                        ),
                    );
                }
                *this.must_yield = true;
                *this.current_budget = *this.budget;
            }
        };

        res
    }
}

fn maybe_log(
    logger: &Option<MaybeOwned<'_, Logger>>,
    location: &slog::RecordLocation,
    fmt: &Arguments<'_>,
) {
    if let Some(logger) = &logger {
        logger.log(&Record::new(
            &slog::RecordStatic {
                location,
                level: slog::Level::Warning,
                tag: "futures_watchdog",
            },
            fmt,
            slog::b!(),
        ));
    }
}

#[cfg(test)]
mod test {
    use futures::stream::StreamExt;

    use super::*;

    #[test]
    fn test_yield_happens() {
        let stream = futures::stream::repeat(()).inspect(|_| {
            // Simulate CPU work
            std::thread::sleep(Duration::from_millis(1));
        });

        let stream =
            YieldPeriodically::new(stream, location_for_test(), Duration::from_millis(100));

        futures::pin_mut!(stream);

        let now = Instant::now();

        let waker = futures::task::noop_waker();
        let mut cx = futures::task::Context::from_waker(&waker);

        while stream.as_mut().poll_next(&mut cx).is_ready() {
            assert!(
                now.elapsed() < Duration::from_millis(200),
                "Stream did not yield in time"
            );
        }

        let now = Instant::now();
        let mut did_unpause = false;

        while stream.as_mut().poll_next(&mut cx).is_ready() {
            did_unpause = true;

            assert!(
                now.elapsed() < Duration::from_millis(200),
                "Stream did not yield in time"
            );
        }

        assert!(did_unpause, "Stream did not unpause");
    }

    #[tokio::test]
    async fn test_yield_registers_for_wakeup() {
        // This will hang if the stream doesn't register
        let stream = futures::stream::repeat(())
            .inspect(|_| {
                // Simulate CPU work
                std::thread::sleep(Duration::from_millis(1));
            })
            .take(30);

        let stream = YieldPeriodically::new(stream, location_for_test(), Duration::from_millis(10));
        stream.collect::<Vec<_>>().await;
    }

    #[track_caller]
    fn location_for_test() -> slog::RecordLocation {
        let location = std::panic::Location::caller();
        slog::RecordLocation {
            file: location.file(),
            line: location.line(),
            column: location.column(),
            function: "",
            module: "",
        }
    }
}
