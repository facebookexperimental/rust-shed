/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Result;
use openssl::ssl::SslAcceptorBuilder;
use slog::Logger;

use crate::SslConfig;

impl SslConfig {
    /// Creates the tls acceptor builder
    pub fn tls_acceptor_builder(self, _: Logger) -> Result<SslAcceptorBuilder> {
        self.inner_tls_acceptor_builder()
    }
}
