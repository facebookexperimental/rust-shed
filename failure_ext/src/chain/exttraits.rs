/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use futures::{Future, Poll, Stream};

use super::Chain;
use anyhow::Error;
use std::error::Error as StdError;

/// Dummy types to distinguish different trait implementations, since we can't do
/// a blanket implementation for all `F: StdError` without getting conherence
/// rule failures for other types which might implement `StdError` in future.
mod markers {
    /// Any F where F: StdError
    pub enum MarkerFail {}
    /// Error
    pub enum MarkerError {}
    /// Result<T, F> where F: StdError
    pub enum MarkerResultFail {}
    /// Result<T, Error>
    pub enum MarkerResultError {}
    /// Future<Error=F> where F: StdError
    pub enum MarkerFutureFail {}
    /// Future<Error=Error>
    pub enum MarkerFutureError {}
    /// Stream<Error=F> where F: StdError
    pub enum MarkerStreamFail {}
    /// Stream<Error=Error>
    pub enum MarkerStreamError {}
    /// Chain for F: StdError
    pub enum MarkerChainFail {}
    /// Chain for Error
    pub enum MarkerChainError {}
}
pub use markers::*;

/// Extension of Error to wrap an error in a higher-level error. This is similar to
/// anyhow::Context, but it is explicitly intended to maintain causal chains of errors.
pub trait ChainExt<MARKER, ERR> {
    /// The resulting type of chaining an error
    type Chained;

    /// Main method of [chain][self::super] module, it let's you chain errors
    /// i.e. add context to them
    fn chain_err(self, outer_err: ERR) -> Self::Chained;
}

impl<ERR> ChainExt<MarkerError, ERR> for Error {
    type Chained = Chain<ERR>;

    fn chain_err(self, err: ERR) -> Chain<ERR> {
        Chain::with_error(err, self)
    }
}

impl<F, ERR> ChainExt<MarkerFail, ERR> for F
where
    F: StdError + Send + Sync + 'static,
{
    type Chained = Chain<ERR>;

    fn chain_err(self, err: ERR) -> Chain<ERR> {
        Chain::with_fail(err, self)
    }
}

impl<T, ERR> ChainExt<MarkerResultError, ERR> for Result<T, Error> {
    type Chained = Result<T, Chain<ERR>>;

    fn chain_err(self, err: ERR) -> Result<T, Chain<ERR>> {
        self.map_err(|cause| Chain::with_error(err, cause))
    }
}

impl<T, F, ERR> ChainExt<MarkerResultFail, ERR> for Result<T, F>
where
    F: StdError + Send + Sync + 'static,
{
    type Chained = Result<T, Chain<ERR>>;

    fn chain_err(self, err: ERR) -> Result<T, Chain<ERR>> {
        self.map_err(|cause| Chain::with_fail(err, cause))
    }
}

type ChainFn<E, ERR> = dyn FnOnce(E) -> Chain<ERR> + Send + 'static;

/// The result of chaining an error to [futures::Future]
pub struct ChainFuture<F, ERR>
where
    F: Future,
{
    chain: Option<Box<ChainFn<F::Error, ERR>>>,
    future: F,
}

impl<F, ERR> Future for ChainFuture<F, ERR>
where
    F: Future,
{
    type Item = F::Item;
    type Error = Chain<ERR>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.future.poll() {
            Err(err) => {
                let f = self
                    .chain
                    .take()
                    .expect("ChainFuture called after error completion");
                Err(f(err))
            }
            Ok(ok) => Ok(ok),
        }
    }
}

impl<F, ERR> ChainExt<MarkerFutureError, ERR> for F
where
    F: Future<Error = Error>,
    ERR: Send + 'static,
{
    type Chained = ChainFuture<F, ERR>;

    fn chain_err(self, err: ERR) -> ChainFuture<F, ERR> {
        ChainFuture {
            chain: Some(Box::new(move |cause| Chain::with_error(err, cause))),
            future: self,
        }
    }
}

impl<F, ERR> ChainExt<MarkerFutureFail, ERR> for F
where
    F: Future,
    F::Error: StdError + Send + Sync + 'static,
    ERR: Send + 'static,
{
    type Chained = ChainFuture<F, ERR>;

    fn chain_err(self, err: ERR) -> ChainFuture<F, ERR> {
        ChainFuture {
            chain: Some(Box::new(move |cause| Chain::with_fail(err, cause))),
            future: self,
        }
    }
}

/// The result of chaining an error to [futures::Stream]
pub struct ChainStream<S, ERR>
where
    S: Stream,
{
    chain: Option<Box<ChainFn<S::Error, ERR>>>,
    stream: S,
}

impl<S, ERR> Stream for ChainStream<S, ERR>
where
    S: Stream,
{
    type Item = S::Item;
    type Error = Chain<ERR>;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.stream.poll() {
            Err(err) => {
                let f = self
                    .chain
                    .take()
                    .expect("ChainFuture called after error completion");
                Err(f(err))
            }
            Ok(ok) => Ok(ok),
        }
    }
}

impl<S, ERR> ChainExt<MarkerStreamError, ERR> for S
where
    S: Stream<Error = Error>,
    ERR: Send + 'static,
{
    type Chained = ChainStream<S, ERR>;

    fn chain_err(self, err: ERR) -> ChainStream<S, ERR> {
        ChainStream {
            chain: Some(Box::new(move |cause| Chain::with_error(err, cause))),
            stream: self,
        }
    }
}

impl<S, ERR> ChainExt<MarkerStreamFail, ERR> for S
where
    S: Stream,
    S::Error: StdError + Send + Sync + 'static,
    ERR: Send + 'static,
{
    type Chained = ChainStream<S, ERR>;

    fn chain_err(self, err: ERR) -> ChainStream<S, ERR> {
        ChainStream {
            chain: Some(Box::new(move |cause| Chain::with_fail(err, cause))),
            stream: self,
        }
    }
}
