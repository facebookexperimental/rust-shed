/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(fbcode_build), allow(unused_crate_dependencies))]

//! Crate defining basic trates and structures for handing fb303 thrift services

use thiserror::Error;

/// Services Error type.
#[derive(Error, Debug)]
pub enum ServicesError {
    /// Unknown C++ exception
    #[error("Unknown C++ exception")]
    CxxException(#[from] cxx::Exception),
}

#[cfg(fbcode_build)]
mod facebook;

/// Status of this service. This mirrors `fb303::fb_status`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FbStatus {
    /// Service is dead
    Dead,
    /// Service is starting
    Starting,
    /// Service is alive and well
    Alive,
    /// Service is in the process of stopping
    Stopping,
    /// Service is already stopped
    Stopped,
    /// Service is alive, but something is wrong with it
    Warning,
}

/// Trait to be implemented by a services supporting fb303.
pub trait Fb303Service: Send + Sync {
    /// Use the name as defined in Thrift here for easier recognizability.
    /// XXX: note that this is Sync and has &self methods because it can be accessed by multiple
    /// threads at the same time. It might be possible to relax these constraints.
    #[allow(non_snake_case)]
    fn getStatus(&self) -> FbStatus;
}

/// A default Fb303Service that just returns Alive.
pub struct AliveService;

impl AliveService {
    /// Create a new service that can be passed into `run_service_framework`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Box<dyn Fb303Service> {
        Box::new(AliveService)
    }
}

impl Fb303Service for AliveService {
    fn getStatus(&self) -> FbStatus {
        FbStatus::Alive
    }
}
