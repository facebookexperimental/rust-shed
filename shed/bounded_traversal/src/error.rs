/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BoundedTraversalError {
    #[error("Programming error at {file}:{line}: {desc}")]
    ProgrammingError {
        desc: String,
        file: &'static str,
        line: u32,
    },
}

macro_rules! programming_error {
    ( $( $args:tt )* ) => {
        $crate::error::BoundedTraversalError::ProgrammingError {
            desc: format!( $( $args )* ),
            file: file!(),
            line: line!(),
        }
    };
}
