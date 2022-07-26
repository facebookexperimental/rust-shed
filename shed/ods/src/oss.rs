/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! See the [send_data_to_ods] documentation

use std::collections::HashMap;

use anyhow::Result;

/// Sends data to ODS.
pub async fn send_data_to_ods(
    _fb: fbinit::FacebookInit,
    _entity: String,
    _agg_entities: Vec<String>,
    _values: HashMap<String, f64>,
    _interval: i32,
    _category: String,
) -> Result<()> {
    Ok(())
}
