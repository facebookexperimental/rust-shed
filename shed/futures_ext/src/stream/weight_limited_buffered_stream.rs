/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use futures::{
    future::BoxFuture,
    ready, stream,
    task::{Context, Poll},
    Future, FutureExt, Stream, StreamExt,
};
use pin_project::pin_project;
use std::pin::Pin;

/// Params for [crate::FbStreamExt::buffered_weight_limited] and [WeightLimitedBufferedStream]
pub struct BufferedParams {
    /// Limit for the sum of weights in the [WeightLimitedBufferedStream] stream
    pub weight_limit: u64,
    /// Limit for size of buffer in the [WeightLimitedBufferedStream] stream
    pub buffer_size: usize,
}

/// Like [stream::Buffered], but can also limit number of futures in a buffer by "weight".
#[pin_project]
pub struct WeightLimitedBufferedStream<S, I> {
    #[pin]
    queue: stream::FuturesOrdered<BoxFuture<'static, (I, u64)>>,
    current_weight: u64,
    weight_limit: u64,
    max_buffer_size: usize,
    #[pin]
    stream: stream::Fuse<S>,
}

impl<S, I> WeightLimitedBufferedStream<S, I>
where
    S: Stream,
{
    /// Create a new instance that will be configured using the `params` provided
    pub fn new(params: BufferedParams, stream: S) -> Self {
        Self {
            queue: stream::FuturesOrdered::new(),
            current_weight: 0,
            weight_limit: params.weight_limit,
            max_buffer_size: params.buffer_size,
            stream: stream.fuse(),
        }
    }
}

impl<S, Fut, I: 'static> Stream for WeightLimitedBufferedStream<S, I>
where
    S: Stream<Item = (Fut, u64)>,
    Fut: Future<Output = I> + Send + 'static,
{
    type Item = I;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // First up, try to spawn off as many futures as possible by filling up
        // our slab of futures.
        while this.queue.len() < *this.max_buffer_size && this.current_weight < this.weight_limit {
            let future = match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some((f, weight))) => {
                    *this.current_weight += weight;
                    f.map(move |val| (val, weight)).boxed()
                }
                Poll::Ready(None) | Poll::Pending => break,
            };

            this.queue.push(future);
        }

        // Try polling a new future
        if let Some((val, weight)) = ready!(this.queue.poll_next(cx)) {
            *this.current_weight -= weight;
            return Poll::Ready(Some(val));
        }

        // If we've gotten this far, then there are no events for us to process
        // and nothing was ready, so figure out if we're not done yet or if
        // we've reached the end.
        if this.stream.is_done() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use futures::future;
    use futures::stream;
    use futures::{future::BoxFuture, stream::BoxStream, FutureExt, StreamExt};

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    type TestStream = BoxStream<'static, (BoxFuture<'static, ()>, u64)>;

    fn create_stream() -> (Arc<AtomicUsize>, TestStream) {
        let s: TestStream = stream::iter(vec![
            (future::ready(()).boxed(), 100),
            (future::ready(()).boxed(), 2),
            (future::ready(()).boxed(), 7),
        ])
        .boxed();

        let counter = Arc::new(AtomicUsize::new(0));

        (
            counter.clone(),
            s.inspect({
                move |_val| {
                    counter.fetch_add(1, Ordering::SeqCst);
                }
            })
            .boxed(),
        )
    }

    #[tokio::test]
    async fn test_too_much_weight_to_do_in_one_go() {
        let (counter, s) = create_stream();
        let params = BufferedParams {
            weight_limit: 10,
            buffer_size: 10,
        };
        let s = WeightLimitedBufferedStream::new(params, s);

        if let (Some(()), s) = s.into_future().await {
            assert_eq!(counter.load(Ordering::SeqCst), 1);
            assert_eq!(s.collect::<Vec<()>>().await.len(), 2);
            assert_eq!(counter.load(Ordering::SeqCst), 3);
        } else {
            panic!("Stream did not produce even a single value");
        }
    }

    #[tokio::test]
    async fn test_all_in_one_go() {
        let (counter, s) = create_stream();
        let params = BufferedParams {
            weight_limit: 200,
            buffer_size: 10,
        };
        let s = WeightLimitedBufferedStream::new(params, s);

        if let (Some(()), s) = s.into_future().await {
            assert_eq!(counter.load(Ordering::SeqCst), 3);
            assert_eq!(s.collect::<Vec<()>>().await.len(), 2);
            assert_eq!(counter.load(Ordering::SeqCst), 3);
        } else {
            panic!("Stream did not produce even a single value");
        }
    }

    #[tokio::test]
    async fn test_too_much_items_to_do_in_one_go() {
        let (counter, s) = create_stream();
        let params = BufferedParams {
            weight_limit: 1000,
            buffer_size: 2,
        };
        let s = WeightLimitedBufferedStream::new(params, s);

        if let (Some(()), s) = s.into_future().await {
            assert_eq!(counter.load(Ordering::SeqCst), 2);
            assert_eq!(s.collect::<Vec<()>>().await.len(), 2);
            assert_eq!(counter.load(Ordering::SeqCst), 3);
        } else {
            panic!("Stream did not produce even a single value");
        }
    }
}
