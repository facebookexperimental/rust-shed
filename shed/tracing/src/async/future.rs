/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use super::{get_task_id, EventId, Traced};
use crate::context::{TraceContext, TraceContextInner};
use chrome_trace::{Args, Event, Phase};
use futures::{Async, Future, Poll};
use maplit::hashmap;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};
use time_ext::DurationExt;

pub struct TracedFuture<F> {
    inner: F,
    context: Weak<TraceContextInner>,
    name: String,
    id: Option<usize>,
    /// scope is used to link AsyncBegin and AsyncEnd events together
    scope: usize,
    args: Option<Args>,
    poll_count: u64,
    poll_time: Duration,
}

impl<F> TracedFuture<F> {
    fn new(
        future: F,
        context: &TraceContext,
        name: String,
        args: Args,
        id: Option<usize>,
        scope: usize,
    ) -> Self {
        Self {
            inner: future,
            context: Arc::downgrade(&context.inner),
            name,
            id,
            scope,
            args: Some(args),
            poll_count: 0,
            poll_time: Duration::from_secs(0),
        }
    }

    fn log_first_poll(&mut self) {
        if self.id.is_none() {
            self.id = Some(get_task_id())
        }

        let context = match self.context.upgrade() {
            Some(ctx) => ctx,
            None => return,
        };

        context.add_event(Event {
            tid: get_task_id() as u64,
            id: self.id.map(|id| id.to_string()),
            scope: Some(self.scope.to_string()),
            args: self
                .args
                .take()
                .expect("The args for tracing were already taken"),
            ..Event::now(&self.name, Phase::AsyncBegin, &context.epoch)
        });
    }

    fn log_completion(&mut self) {
        let context = match self.context.upgrade() {
            Some(ctx) => ctx,
            None => return,
        };

        context.add_event(Event {
            tid: get_task_id() as u64,
            id: self.id.map(|id| id.to_string()),
            scope: Some(self.scope.to_string()),
            args: hashmap! {
                "poll_count".to_owned() => self.poll_count.into(),
                "poll_time".to_owned() => self.poll_time.as_micros_unchecked().into(),
            },
            ..Event::now(&self.name, Phase::AsyncEnd, &context.epoch)
        });
    }
}

impl<F: Future> Future for TracedFuture<F> {
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.poll_count == 0 {
            self.log_first_poll();
        }

        let poll_start = Instant::now();
        let poll = self.inner.poll();
        self.poll_time += poll_start.elapsed();

        match &poll {
            Ok(Async::Ready(_)) | Err(_) => {
                self.log_completion();
            }
            Ok(Async::NotReady) => {}
        }

        self.poll_count += 1;
        poll
    }
}

/// Dummy type used for the sole purpose of preventing overlapping implementations of
/// the `Traced<T>` trait for `Future`s and `Stream`s.
pub enum FutureMarker {}

impl<F: Future> Traced<FutureMarker> for F {
    type Wrapper = TracedFuture<Self>;

    fn traced<N: ToString>(
        self,
        context: &TraceContext,
        name: N,
        args: Option<Args>,
    ) -> Self::Wrapper {
        TracedFuture::new(
            self,
            context,
            name.to_string(),
            args.unwrap_or_default(),
            None,
            EventId::new().id,
        )
    }

    fn traced_with_id<N: ToString>(
        self,
        context: &TraceContext,
        name: N,
        args: Option<Args>,
        id: EventId,
    ) -> Self::Wrapper {
        TracedFuture::new(
            self,
            context,
            name.to_string(),
            args.unwrap_or_default(),
            Some(id.id),
            EventId::new().id,
        )
    }
}
