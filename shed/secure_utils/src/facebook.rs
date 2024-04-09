// (c) Meta Platforms, Inc. and affiliates. Confidential and proprietary.

//! Module for facebook specific TLS parts

use std::mem;
use std::path::Path;
use std::ptr;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use anyhow::bail;
use anyhow::Context;
use anyhow::Error;
use anyhow::Result;
use libc::c_int;
use libc::c_uchar;
use libc::c_void;
use openssl::nid::Nid;
#[allow(deprecated)]
use openssl::pkcs12::ParsedPkcs12;
use openssl::ssl::SslAcceptorBuilder;
use openssl_sys::EVP_CIPHER_CTX;
use openssl_sys::HMAC_CTX;
use openssl_sys::SSL;
use openssl_sys::SSL_CTX;
use openssl_sys::{self as ffi};
use serde::Deserialize;
use slog::error;
use slog::info;
use slog::Logger;

use crate::build_identity;
use crate::SslConfig;

// TODO(T37711609): this should be stored in the OpenSSL SSL struct via the ex_data functions
static TLS_TICKETS: Mutex<Vec<TlsTicket>> = Mutex::new(vec![]);

const UPDATE_TICKET_SEEDS_INTERVAL: u64 = 300; // in seconds, update seed every 5 min
const TICKET_PART_SIZE: usize = 16;

/// Constant manually defined since it's not exposed in openssl_sys bindings, verified that
/// this constant is the same across the various OpenSSL verions used: 1.0.2 to 1.1.1
///
/// from OpenSSL/include/openssl/ssl.h
const SSL_MAX_SID_CTX_LENGTH: c_int = 32;

impl SslConfig {
    /// Sets properties and handlers on our Ssl handler to handle
    /// with facebook specific TLS setup
    pub fn tls_acceptor_builder(self, logger: Logger) -> Result<SslAcceptorBuilder> {
        // Set up the context identifier, which is an OpenSSL requirement for doing TlS
        // resumption in combination with client certificates.
        let pkcs12 =
            build_identity(&self.cert, &self.private_key).context("failed to build pkcs12")?;
        #[allow(deprecated)]
        let ParsedPkcs12 { cert, .. } = pkcs12;

        let subject = cert.subject_name();
        let cn = subject
            .entries_by_nid(Nid::COMMONNAME)
            .next()
            .ok_or_else(|| Error::msg("cert does not contain common name"))?;
        let session_context = cn
            .data()
            .as_slice()
            .chunks(SSL_MAX_SID_CTX_LENGTH as usize)
            .next()
            .ok_or_else(|| Error::msg("cert does not contain common name"))?;

        let tls_seed_path = self.tls_seed_path.clone();

        let mut acceptor = self.inner_tls_acceptor_builder()?;
        acceptor.set_session_id_context(session_context)?;

        if let Some(tls_seed_path) = tls_seed_path {
            let p = Path::new(&tls_seed_path);
            if !p.exists() {
                bail!("Can't initialise tls seeds. File does not exist: {:?}", p);
            }
            update_ticket_seeds(&tls_seed_path)?;
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(UPDATE_TICKET_SEEDS_INTERVAL));
                    match update_ticket_seeds(&tls_seed_path) {
                        Ok(_) => info!(logger, "updated TLS ticket seeds"),
                        Err(_) => error!(logger, "failed to update TLS ticket seeds"),
                    };
                }
            });

            unsafe {
                ssl_ctx_set_tlsext_ticket_key_cb(acceptor.as_ptr(), Some(handle_tls_ticket));
            }
        }

        Ok(acceptor)
    }
}

/// All current, past and future tls tickets for 72 hours
#[derive(Deserialize)]
pub struct TicketSeeds {
    old: Vec<String>,
    new: Vec<String>,
    current: Vec<String>,
}

/// Different parts of FB TLS Tickets
#[derive(Clone, Copy, Default, Debug)]
pub struct TlsTicket {
    /// Name of TLS Tickets key
    pub name: [c_uchar; TICKET_PART_SIZE],
    /// AES Key for TLS Tickets
    pub aes_key: [c_uchar; TICKET_PART_SIZE],
    /// HMAC Key for TlS tickets
    pub hmac_key: [c_uchar; TICKET_PART_SIZE],
}

impl TlsTicket {
    fn from_hex(data: String) -> Result<TlsTicket> {
        let bytes = hex::decode(data)?;
        if bytes.len() < (3 * TICKET_PART_SIZE) {
            bail!("tls ticket seed not long enough");
        }

        let mut parts = bytes.chunks_exact(TICKET_PART_SIZE);
        let mut t: TlsTicket = Default::default();
        t.name
            .copy_from_slice(parts.next().expect("missing name in tls ticket"));
        t.aes_key
            .copy_from_slice(parts.next().expect("missing aes key in tls ticket"));
        t.hmac_key
            .copy_from_slice(parts.next().expect("missing hmac key in tls ticket"));

        Ok(t)
    }
}

fn update_ticket_seeds<P: AsRef<Path>>(ticket_seeds_file: P) -> Result<bool> {
    let mut seeds = read_ticket_seeds(ticket_seeds_file)?;

    // Explicitly unwrap here, since our failure_ext isn't thread-safe
    let mut ticket_seeds = TLS_TICKETS.lock().expect("lock poisoned");
    ticket_seeds.clear();
    ticket_seeds.append(&mut seeds);

    Ok(true)
}

/// Read TLS tickets from filesystem
pub fn read_ticket_seeds<P: AsRef<Path>>(ticket_seeds_file: P) -> Result<Vec<TlsTicket>> {
    let f = std::fs::File::open(ticket_seeds_file)?;
    let seeds: TicketSeeds = serde_json::from_reader(f)?;

    let mut tls_tickets: Vec<TlsTicket> = Vec::new();
    for s in seeds.current {
        tls_tickets.push(TlsTicket::from_hex(s)?);
    }
    for s in seeds.new {
        tls_tickets.push(TlsTicket::from_hex(s)?);
    }
    for s in seeds.old {
        tls_tickets.push(TlsTicket::from_hex(s)?);
    }

    Ok(tls_tickets)
}

fn get_encryption_key() -> Result<TlsTicket> {
    // Explicitly unwrap here, since our failure_ext isn't thread-safe
    TLS_TICKETS
        .lock()
        .expect("lock poisoned")
        .first()
        .cloned()
        .ok_or_else(|| Error::msg("no encryption key found"))
}

fn find_decryption_key(name: *mut c_uchar) -> Result<TlsTicket> {
    // Explicitly unwrap here, since our failure_ext isn't thread-safe
    let tickets = TLS_TICKETS.lock().expect("lock poisoned");
    for ticket in tickets.iter() {
        let res = unsafe {
            libc::memcmp(
                ticket.name.as_ptr() as *const libc::c_void,
                name as *const libc::c_void,
                TICKET_PART_SIZE,
            )
        };

        if res == 0 {
            return Ok(*ticket);
        }
    }

    Err(Error::msg("no encryption key found"))
}

/// Implements OpenSSL ticket handler set by [SSL_CTX_set_tlsext_ticket_key_cb][1]
///
/// This is used to make sure that session tickets can be decrypted by all Tasks of
/// a service since encryption keys for tickets are seeded to all Tasks.
///
/// [1]: https://www.openssl.org/docs/man1.0.2/ssl/SSL_CTX_set_tlsext_ticket_key_cb.html
#[no_mangle]
unsafe extern "C" fn handle_tls_ticket(
    _ssl: *mut SSL,
    name: *mut c_uchar,
    iv: *mut c_uchar,
    ectx: *mut EVP_CIPHER_CTX,
    hctx: *mut HMAC_CTX,
    enc: i32,
) -> i32 {
    if enc > 0 {
        let key = match get_encryption_key() {
            Ok(key) => key,
            Err(_) => return -1, // Signal error and hang up connection
        };

        ptr::copy_nonoverlapping(key.name.as_ptr(), name, TICKET_PART_SIZE);
        if ffi::RAND_bytes(iv, TICKET_PART_SIZE as i32) <= 0 {
            // Signal error and hang up connection
            return -1;
        }

        ffi::EVP_CipherInit_ex(
            ectx,
            ffi::EVP_aes_128_cbc(),
            ptr::null_mut(),
            key.aes_key.as_ptr(),
            iv,
            1,
        );
        ffi::HMAC_Init_ex(
            hctx,
            key.hmac_key.as_ptr() as *const c_void,
            16,
            ffi::EVP_sha256(),
            ptr::null_mut(),
        );

        // Proceed with the set encryption parameters
        1
    } else {
        let key = match find_decryption_key(name) {
            Ok(key) => key,
            Err(_) => return 0, // Perform TLS renegotiation
        };

        ffi::HMAC_Init_ex(
            hctx,
            key.hmac_key.as_ptr() as *const c_void,
            TICKET_PART_SIZE as i32,
            ffi::EVP_sha256(),
            ptr::null_mut(),
        );
        ffi::EVP_CipherInit_ex(
            ectx,
            ffi::EVP_aes_128_cbc(),
            ptr::null_mut(),
            key.aes_key.as_ptr(),
            iv,
            0,
        );

        // Proceed with the set encryption parameters
        1
    }
}

/// Constant manually defined since it's not exposed in openssl_sys bindings, verified that
/// this constant is the same across the various OpenSSL verions used: 1.0.2 to 1.1.1
///
/// from OpenSSL/include/openssl/ssl.h
const SSL_CTRL_SET_TLSEXT_TICKET_KEY_CB: c_int = 72;

unsafe fn ssl_ctx_set_tlsext_ticket_key_cb(
    ctx: *mut SSL_CTX,
    cb: Option<
        unsafe extern "C" fn(
            *mut SSL,
            *mut c_uchar,
            *mut c_uchar,
            *mut EVP_CIPHER_CTX,
            *mut HMAC_CTX,
            c_int,
        ) -> c_int,
    >,
) {
    // OpenSSL macro SSL_CTX_set_tlsext_ticket_key_cb isn't available in openssl-sys, so
    // directly do what the macro would have done. mem::transmute is used here to forcefully pass
    // our callback because openssl uses a generic callback handler for all different callbacks
    // with different function signatures. This is consistent with how rust-openssl bindings deal
    // with similar issues.
    #[allow(deprecated)]
    ffi::SSL_CTX_callback_ctrl(ctx, SSL_CTRL_SET_TLSEXT_TICKET_KEY_CB, mem::transmute(cb));
}
