/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use serde_derive::Deserialize;

use crate::ConfigHandle;
use crate::ConfigStore;
use crate::ModificationTime;
use crate::TestSource;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct TestConfig {
    value: i64,
}

fn get_test_handle(store: &ConfigStore, path: &str) -> Result<ConfigHandle<TestConfig>> {
    store.get_config_handle_DEPRECATED(path.to_owned())
}

#[test]
fn test_config_store() {
    let test_source = {
        let test_source = TestSource::new();
        test_source.insert_config(
            "some1",
            r#"{ "value": 1 }"#,
            ModificationTime::UnixTimestamp(1),
        );
        test_source.insert_config(
            "some2",
            r#"{ "value": 2 }"#,
            ModificationTime::UnixTimestamp(1),
        );
        Arc::new(test_source)
    };

    let store = ConfigStore::new(test_source.clone(), Duration::from_millis(2), None);

    let handle1 = get_test_handle(&store, "some1").expect("Failed to get handle1");
    let handle2 = get_test_handle(&store, "some2").expect("Failed to get handle2");

    assert_eq!(*handle1.get(), TestConfig { value: 1 });
    assert_eq!(*handle2.get(), TestConfig { value: 2 });
    assert!(
        get_test_handle(&store, "some4").is_err(),
        "some4 should have not exist"
    );

    test_source.insert_config(
        "some1",
        r#"{ "value": 11 }"#,
        ModificationTime::UnixTimestamp(1),
    );
    test_source.insert_config(
        "some2",
        r#"{ "value": 22 }"#,
        ModificationTime::UnixTimestamp(2),
    );
    test_source.insert_config(
        "some4",
        r#"{ "value": 4 }"#,
        ModificationTime::UnixTimestamp(1),
    );

    let handle1_v2 = get_test_handle(&store, "some1").expect("Failed to get handle1_v2");
    let handle2_v2 = get_test_handle(&store, "some2").expect("Failed to get handle2_v2");
    let handle4 = get_test_handle(&store, "some4").expect("Failed to get handle4");

    assert_eq!(*handle1_v2.get(), TestConfig { value: 11 });
    assert_eq!(*handle2_v2.get(), TestConfig { value: 22 });
    assert_eq!(*handle4.get(), TestConfig { value: 4 });
    // The TestSource::paths_to_refresh still doesn't report any refreshes, so
    // nothing is changed in old handles
    assert_eq!(*handle1.get(), TestConfig { value: 1 });
    assert_eq!(*handle2.get(), TestConfig { value: 2 });

    test_source.insert_to_refresh("some1".to_owned());
    test_source.insert_to_refresh("some2".to_owned());

    // Ensure the updater thread has run
    thread::yield_now();
    thread::sleep(Duration::from_secs(1));

    // handle1 remains the same, because the mod_time has not changed
    assert_eq!(*handle1.get(), TestConfig { value: 1 });
    assert_eq!(*handle2.get(), TestConfig { value: 22 });

    // Test raw configs, too
    let raw_handle = store
        .get_raw_config_handle("some1".to_string())
        .expect("Failed to get raw handle");
    assert_eq!(*raw_handle.get(), r#"{ "value": 11 }"#);
}

#[test]
fn test_config_handle_from_json() {
    let result = ConfigHandle::<TestConfig>::from_json(r#"{ "value": 44 }"#)
        .expect("failed to deserialize json")
        .get();
    assert_eq!(*result, TestConfig { value: 44 });
}
