/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module that helps with dealing of the internal errors of this crate

use thiserror::Error;

/// Required for code in Mononoke that needs to downcast to ServerError to check
/// the code. To be removed after upgrading to mysql_async 0.21+ which drops
/// failure and provides correct std::error::Error impls for its error types.
#[derive(Error, Debug)]
#[error("ERROR {state} ({code}): {message}")]
pub struct ServerError {
    /// Contains [::mysql_async::error::ServerError::code]
    pub code: u16,
    message: String,
    state: String,
}

/// Used to convert a mysql_async error type into [anyhow::Error]
pub fn from_failure(failure: mysql_async::error::Error) -> anyhow::Error {
    match failure {
        mysql_async::error::Error::Server(mysql_async::error::ServerError {
            code,
            message,
            state,
        }) => anyhow::Error::new(ServerError {
            code,
            message,
            state,
        }),
        _ => failure_ext::convert(failure),
    }
}
