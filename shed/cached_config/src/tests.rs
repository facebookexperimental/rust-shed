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
use tokio::time::timeout;

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

#[tokio::test]
async fn test_config_handle_wait_for_latest() {
    let test_source = {
        let test_source = TestSource::new();
        test_source.insert_config(
            "some1",
            r#"{ "value": 1 }"#,
            ModificationTime::UnixTimestamp(1),
        );
        Arc::new(test_source)
    };

    let store = ConfigStore::new(test_source.clone(), Duration::from_millis(2), None);

    let handle1 = get_test_handle(&store, "some1").expect("Failed to get handle1");

    // wait_for_next should not return and instead timeout since the config hasn't been updated.
    assert!(
        timeout(Duration::from_millis(200), handle1.wait_for_next())
            .await
            .is_err()
    );
    // Update the config
    test_source.insert_config(
        "some1",
        r#"{ "value": 11 }"#,
        ModificationTime::UnixTimestamp(2),
    );
    // Mark the config for refresh
    test_source.insert_to_refresh("some1".to_owned());
    // Ensure the updater thread has run
    thread::yield_now();
    thread::sleep(Duration::from_secs(1));

    // Now that the config is updated, wait_for_next should return immediately.
    let val = timeout(Duration::from_millis(200), handle1.wait_for_next()).await;
    // Ensure that we did not timeout and the wait_for_next future terminated.
    assert!(val.is_ok());
    // Ensure that wait_for_next got the latest value
    assert_eq!(*val.expect("Value wasn't ok"), TestConfig { value: 11 });
}
