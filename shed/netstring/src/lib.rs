/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

//! Tokio-based implementation of netstrings
//!
//! [Netstring](http://cr.yp.to/proto/netstrings.txt) is an extremely simple mechanism for
//! delimiting messages in a stream.
//!
//! Each message has the form "7:message," where the initial decimal number is the size of the
//! payload, followed by a ':', then the payload, and a terminating ','. There is no error
//! checking or correction other than the requirement that the message be followed by a comma.

#![deny(warnings)]

use thiserror::Error;

/// Errors that can originate from this crate
#[derive(Clone, Debug, Error)]
pub enum ErrorKind {
    /// Error while decoding netstring
    #[error("{0}")]
    NetstringDecode(&'static str),
}

mod decode;
mod encode;

pub use crate::decode::NetstringDecoder;
pub use crate::encode::NetstringEncoder;
