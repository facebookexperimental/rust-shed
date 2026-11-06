/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::hint::black_box;

use minibench::bench;
use minibench::elapsed;
use mysql_async::prelude::ToValue;
use sql_common::Binary;
use sql_common::mysql::AsSql;

const CHUNK_SIZE: usize = 1024 * 1024;

fn upstream(bytes: &[u8]) -> String {
    bytes.to_value().as_sql(false)
}

fn shed(bytes: &[u8]) -> String {
    AsSql::as_sql(&bytes, false)
}

fn explicit_binary(bytes: &[u8]) -> String {
    AsSql::as_sql(&Binary(bytes), false)
}

fn main() {
    println!("Ordinary values use ToValue; explicit binary parameters borrow the input bytes.");
    println!("Inputs above 1 MiB are split into 1 MiB SQL values.");
    for size in [
        32,
        256,
        4096,
        65536,
        CHUNK_SIZE,
        4 * CHUNK_SIZE,
        16 * CHUNK_SIZE,
    ] {
        let repetitions = if size < CHUNK_SIZE { 100 } else { 1 };
        println!("Input size: {size} bytes; {repetitions} repetitions per measurement.");
        let mut binary = (0..=255).cycle().take(size).collect::<Vec<u8>>();
        binary[0] = 0xff;
        let mut late_invalid = vec![b'a'; size];
        for chunk in late_invalid.chunks_mut(CHUNK_SIZE) {
            *chunk.last_mut().unwrap() = 0xff;
        }

        for (kind, bytes) in [
            ("binary", binary),
            ("late_invalid", late_invalid),
            ("ascii", vec![b'a'; size]),
            ("utf8", "é".repeat(size / 2).into_bytes()),
        ] {
            for chunk in bytes.chunks(CHUNK_SIZE) {
                assert_eq!(upstream(chunk), shed(chunk));
            }
            for (name, format) in [
                ("upstream", upstream as fn(&[u8]) -> String),
                ("shed", shed),
                ("explicit_binary", explicit_binary),
            ] {
                bench(format!("{kind}/{size}/{name}"), || {
                    elapsed(|| {
                        for _ in 0..repetitions {
                            for chunk in bytes.chunks(CHUNK_SIZE) {
                                black_box(format(black_box(chunk)));
                            }
                        }
                    })
                });
            }
        }
    }
}
