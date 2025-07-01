/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::pin::Pin;
use std::time::Duration;

use futures::Future;
use futures::FutureExt;
use futures::Stream;
use futures::StreamExt as _;
use futures::future::BoxFuture;
use futures::stream;
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::BufferedWeighted;
use crate::StreamExt as _;
use crate::traits::WeightedFuture;

#[derive(Clone, Debug, Arbitrary)]
struct TestState {
    #[proptest(strategy = "1usize..64")]
    max_weight: usize,
    #[proptest(strategy = "prop::collection::vec(TestFutureDesc::arbitrary(), 0..512usize)")]
    future_descriptions: Vec<TestFutureDesc>,
}

#[derive(Copy, Clone, Debug, Arbitrary)]
struct TestFutureDesc {
    #[proptest(strategy = "duration_strategy()")]
    start_delay: Duration,
    #[proptest(strategy = "duration_strategy()")]
    delay: Duration,
    #[proptest(strategy = "0usize..8")]
    weight: usize,
}

fn duration_strategy() -> BoxedStrategy<Duration> {
    // Allow for a delay between 0ms and 1000ms uniformly at random.
    (0u64..1000).prop_map(Duration::from_millis).boxed()
}

trait StreamSpec: Arbitrary + Send + Sync + Copy + 'static {
    type Item: Send;
    type CheckState: Default;

    fn create_stream<'a, St>(stream: St, state: &TestState) -> BoxedWeightedStream<'a, ()>
    where
        St: Stream<Item = Self::Item> + Send + 'static;

    fn create_stream_item(
        desc: &TestFutureDesc,
        future: impl Future<Output = ()> + Send + 'static,
    ) -> Self::Item;

    fn check_started(
        check_state: &mut Self::CheckState,
        id: usize,
        desc: &TestFutureDesc,
        state: &TestState,
    );

    fn check_finished(check_state: &mut Self::CheckState, desc: &TestFutureDesc, state: &TestState);
}

trait WeightedStream: Stream {
    fn current_weight(&self) -> usize;
}

impl<St, Fut> WeightedStream for BufferedWeighted<St>
where
    St: Stream<Item = Fut>,
    Fut: WeightedFuture,
{
    fn current_weight(&self) -> usize {
        self.current_weight()
    }
}

type BoxedWeightedStream<'a, Item> = Pin<Box<dyn WeightedStream<Item = Item> + Send + 'a>>;

impl StreamSpec for () {
    type Item = (usize, BoxFuture<'static, ()>);
    type CheckState = WeightedCheckState;

    fn create_stream<'a, St>(stream: St, state: &TestState) -> BoxedWeightedStream<'a, ()>
    where
        St: Stream<Item = Self::Item> + Send + 'static,
    {
        Box::pin(stream.buffered_weighted(state.max_weight))
    }

    fn create_stream_item(
        desc: &TestFutureDesc,
        future: impl Future<Output = ()> + Send + 'static,
    ) -> Self::Item {
        (desc.weight, future.boxed())
    }

    fn check_started(
        check_state: &mut Self::CheckState,
        id: usize,
        desc: &TestFutureDesc,
        state: &TestState,
    ) {
        // last_started_id must be 1 less than id.
        let expected_id = check_state.last_started_id.map_or(0, |id| id + 1);
        assert_eq!(
            expected_id, id,
            "expected future id to start != actual id that started"
        );
        check_state.last_started_id = Some(id);

        // Check that current_weight doesn't go over the limit.
        check_state.current_weight += desc.weight.min(state.max_weight);
        assert!(
            check_state.current_weight <= state.max_weight,
            "current weight {} <= max weight {}",
            check_state.current_weight,
            state.max_weight,
        );
    }

    fn check_finished(
        check_state: &mut Self::CheckState,
        desc: &TestFutureDesc,
        state: &TestState,
    ) {
        check_state.current_weight -= desc.weight.min(state.max_weight);
    }
}

#[derive(Debug, Default)]
struct WeightedCheckState {
    last_started_id: Option<usize>,
    current_weight: usize,
}

// ---
// Tests
// ---

#[test]
fn test_examples() {
    let state = TestState {
        max_weight: 1,
        future_descriptions: vec![TestFutureDesc {
            start_delay: Duration::ZERO,
            delay: Duration::ZERO,
            weight: 0,
        }],
    };
    test_future_queue_impl::<()>(state);
}

proptest! {
    #[test]
    fn proptest_future_queue(state: TestState) {
        test_future_queue_impl::<()>(state)
    }
}

#[derive(Clone, Copy, Debug)]
enum FutureEvent {
    Started(usize, TestFutureDesc),
    Finished(usize, TestFutureDesc),
}

fn test_future_queue_impl<S: StreamSpec>(state: TestState) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("tokio builder succeeded");
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let (future_sender, future_receiver) = tokio::sync::mpsc::unbounded_channel();

    let futures = state
        .future_descriptions
        .iter()
        .enumerate()
        .map(move |(id, desc)| {
            let desc = *desc;
            let sender = sender.clone();
            let future_sender = future_sender.clone();
            async move {
                // First, sleep for this long.
                tokio::time::sleep(desc.start_delay).await;
                // For each description, create a future.
                let delay_fut = async move {
                    // Send the fact that this future started to the mpsc queue.
                    sender
                        .send(FutureEvent::Started(id, desc))
                        .expect("receiver held open by loop");
                    tokio::time::sleep(desc.delay).await;
                    sender
                        .send(FutureEvent::Finished(id, desc))
                        .expect("receiver held open by loop");
                };
                // Errors should never occur here.
                if let Err(err) = future_sender.send(S::create_stream_item(&desc, delay_fut)) {
                    panic!("future_receiver held open by loop: {}", err);
                }
            }
        })
        .collect::<Vec<_>>();
    let combined_future = stream::iter(futures).buffer_unordered(1).collect::<()>();
    runtime.spawn(combined_future);

    // We're going to use future_receiver as a stream.
    let stream = UnboundedReceiverStream::new(future_receiver);

    let mut completed_map = vec![false; state.future_descriptions.len()];
    let mut check_state = S::CheckState::default();

    runtime.block_on(async move {
        // Record values that have been completed in this map.
        let mut stream = S::create_stream(stream, &state);
        let mut receiver_done = false;
        loop {
            tokio::select! {
                // biased ensures that the receiver is drained before the stream is polled. Without
                // it, it's possible that we fail to record the completion of some futures in status_map.
                biased;

                recv = receiver.recv(), if !receiver_done => {
                    match recv {
                        Some(FutureEvent::Started(id, desc)) => {
                            S::check_started(&mut check_state, id, &desc, &state);
                        }
                        Some(FutureEvent::Finished(id, desc)) => {
                            // Record that this value was completed.
                            completed_map[id] = true;
                            S::check_finished(&mut check_state, &desc, &state);
                        }
                        None => {
                            // All futures finished -- going to check for completion in stream.next() below.
                            receiver_done = true;
                        }
                    }
                }
                next = stream.next() => {
                    if next.is_none() {
                        assert_eq!(stream.current_weight(), 0, "all futures complete => current weight is 0");
                        break;
                    }
                }
                else => {
                    tokio::time::advance(Duration::from_millis(1)).await;
                }
            }
        }

        // Check that all futures completed.
        let not_completed: Vec<_> = completed_map
            .iter()
            .enumerate()
            .filter(|(_, v)| !*v).map(|(n, _)| n.to_string())
            .collect();
        if !not_completed.is_empty() {
            let not_completed_ids = not_completed.join(", ");
            panic!("some futures did not complete: {}", not_completed_ids);
        }
    })
}
