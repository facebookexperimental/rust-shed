/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use futures_old::{future, stream, Stream};

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

    let empty: Vec<u32> = vec![];
    let stream = stream::iter_ok(empty.into_iter()).timed(|stats, _: Result<_, &()>| {
        assert!(stats.first_item_time.is_none());
        Ok(())
    });
    runtime.block_on(stream.for_each(|_| Ok(()))).unwrap();
}
