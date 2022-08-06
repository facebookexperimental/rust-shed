/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Crate with useful security utilities

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]
#![cfg_attr(not(fbcode_build), allow(unused_crate_dependencies))]

#[cfg(fbcode_build)]
pub mod facebook;
#[cfg(not(fbcode_build))]
mod oss;

use std::io::Read;
use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use openssl::pkcs12::ParsedPkcs12;
use openssl::pkey::PKey;
use openssl::ssl::SslAcceptor;
use openssl::ssl::SslAcceptorBuilder;
use openssl::ssl::SslMethod;
use openssl::ssl::SslVerifyMode;
use openssl::x509::X509;
use slog::Logger;

/// Certificates for the TLS acceptor
#[derive(Clone, Debug)]
pub struct SslConfig {
    ca_pem: String,
    cert: String,
    private_key: String,
    #[allow(unused)] // TODO unused warning after rustc upgrade
    tls_seed_path: Option<PathBuf>,
}

impl SslConfig {
    /// Create a new instance of SslConfig
    pub fn new(
        ca_pem: impl Into<String>,
        cert: impl Into<String>,
        private_key: impl Into<String>,
        tls_seed_path: Option<impl Into<PathBuf>>,
    ) -> Self {
        Self {
            ca_pem: ca_pem.into(),
            cert: cert.into(),
            private_key: private_key.into(),
            tls_seed_path: tls_seed_path.map(|x| x.into()),
        }
    }

    /// Builds the tls acceptor
    pub fn build_tls_acceptor(self, logger: Logger) -> Result<SslAcceptor> {
        Ok(self.tls_acceptor_builder(logger)?.build())
    }

    /// Creates a acceptor builder with Ssl security configs pre set.
    fn inner_tls_acceptor_builder(self) -> Result<SslAcceptorBuilder> {
        let mut acceptor = SslAcceptor::mozilla_intermediate_v5(SslMethod::tls())?;

        let pkcs12 =
            build_identity(self.cert, self.private_key).context("failed to build pkcs12")?;
        acceptor.set_certificate(&pkcs12.cert)?;
        acceptor.set_private_key(&pkcs12.pkey)?;

        // Set up client authentication via root certificate
        for cert in read_x509_stack(self.ca_pem)? {
            acceptor.cert_store_mut().add_cert(cert)?;
        }
        acceptor.set_verify(SslVerifyMode::PEER | SslVerifyMode::FAIL_IF_NO_PEER_CERT);

        Ok(acceptor)
    }
}

/// Read certificate and private key data from pem files and convert it into native_tls::Identity
/// archive
pub fn build_identity(
    cert_pem_file: impl AsRef<Path>,
    private_key_pem_file: impl AsRef<Path>,
) -> Result<ParsedPkcs12> {
    let cert = read_x509(cert_pem_file)?;

    // Read PEM-formatted input file as bytes.
    let key_pem = read_bytes(private_key_pem_file)?;

    // Parse PEM-encoded data into appropriate formats for each item.
    let pkey = PKey::private_key_from_pem(&key_pem)?;

    Ok(ParsedPkcs12 {
        pkey,
        cert,
        chain: None,
    })
}

/// Read certificate pem file and decode it as X509
pub fn read_x509<P: AsRef<Path>>(cert_pem_file: P) -> Result<X509> {
    // Read PEM-formatted input file as bytes.
    let cert_pem = read_bytes(cert_pem_file)?;
    let cert = X509::from_pem(&cert_pem)?;
    Ok(cert)
}

/// Read certificate pem file and decode it as stack of X509
pub fn read_x509_stack<P: AsRef<Path>>(cert_pem_file: P) -> Result<Vec<X509>> {
    let cert_pem = read_bytes(cert_pem_file)?;
    let certs = X509::stack_from_pem(&cert_pem)?;
    Ok(certs)
}

fn read_bytes<T: AsRef<Path>>(path: T) -> Result<Vec<u8>> {
    let path = path.as_ref();
    (|| -> Result<_> {
        let mut f = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(buf)
    })()
    .with_context(|| format!("While reading file {}", path.display()))
}
