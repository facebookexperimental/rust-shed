/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! Tokio-based implementation of netstrings
//!
//! [Netstring](http://cr.yp.to/proto/netstrings.txt) is an extremely simple mechanism for
//! delimiting messages in a stream.
//!
//! Each message has the form "7:message," where the initial decimal number is the size of the
//! payload, followed by a ':', then the payload, and a terminating ','. There is no error
//! checking or correction other than the requirement that the message be followed by a comma.

#![deny(warnings)]

use failure_ext as failure;

mod errors {
    pub use crate::failure::{Error, Result};
    use thiserror::Error;

    #[derive(Clone, Debug, Error)]
    pub enum ErrorKind {
        #[error("{0}")]
        NetstringDecode(&'static str),
    }
}
pub use crate::errors::*;

mod decode;
mod encode;

pub use crate::decode::NetstringDecoder;
pub use crate::encode::NetstringEncoder;
