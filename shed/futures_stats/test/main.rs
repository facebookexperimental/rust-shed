// (c) Facebook, Inc. and its affiliates. Confidential and proprietary.

use futures::{future, stream, Stream};

use futures_stats::{Timed, TimedStreamTrait};

fn main() {
    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    let fut = future::lazy(|| {
        println!("future polled");
        Ok(())
    })
    .timed(|stats, _: Result<&(), &()>| {
        println!("{:#?}", stats);
        Ok(())
    });
    runtime.block_on(fut).unwrap();

    let stream = stream::iter_ok([1, 2, 3].iter()).timed(|stats, _: Result<_, &()>| {
        println!("{:#?}", stats);
        Ok(())
    });
    runtime.block_on(stream.for_each(|_| Ok(()))).unwrap();
}
