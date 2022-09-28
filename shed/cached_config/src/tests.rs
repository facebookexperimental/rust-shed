/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::sync::atomic::AtomicI64;
use std::sync::atomic::Ordering;
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

const SLEEP_TIME_MS: u64 = 50;

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
    thread::sleep(Duration::from_millis(SLEEP_TIME_MS));

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
async fn test_config_update_watcher_basic() {
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
    let watcher = handle1.watcher();
    // Validate if fetching Config Update Watcher for dynamic sourced config works
    // successfully
    assert!(watcher.is_ok());
    let mut watcher = watcher.unwrap();

    // wait_for_next should not return and instead timeout since the config hasn't
    // been updated post watcher creation.
    assert!(
        timeout(Duration::from_millis(200), watcher.wait_for_next())
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
    thread::sleep(Duration::from_millis(SLEEP_TIME_MS));

    // Now that the config is updated, wait_for_next should return immediately.
    let val = timeout(Duration::from_millis(10), watcher.wait_for_next()).await;
    // Ensure that we did not timeout and the wait_for_next future terminated.
    assert!(val.is_ok());
    let val = val.unwrap();
    // Ensure that the watcher could successfully notice the config change
    // without erroring out.
    assert!(val.is_ok());
    let val = val.unwrap();
    // Ensure that wait_for_next got the latest value
    assert_eq!(*val, TestConfig { value: 11 });
}

#[tokio::test]
async fn test_config_update_watcher_wait_for_next() {
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
    let watcher = handle1.watcher();
    let flag_val = Arc::new(AtomicI64::new(1));

    // Validate if fetching Config Update Watcher for dynamic sourced config works
    // successfully
    assert!(watcher.is_ok());
    let mut watcher = watcher.unwrap();

    let task_handle = tokio::spawn({
        let flag_val = flag_val.clone();
        async move {
            let mut iteration = 1;
            loop {
                let new_val = watcher.wait_for_next().await.unwrap().value * iteration;
                flag_val.store(new_val, Ordering::Relaxed);
                iteration += 1;
            }
        }
    });
    // Give the async closure some time to run since tests are executed
    // in single threaded environment.
    tokio::time::sleep(Duration::from_millis(SLEEP_TIME_MS)).await;
    // Ensure that the flag_val has not changed inspite of the watcher waiting
    // for the next configuration value. Given that the underlying config hasn't
    // changed, the watcher should be sleeping and thus flag_val should not be updated.
    assert_eq!(flag_val.load(Ordering::Relaxed), 1);

    // Update the config
    test_source.insert_config(
        "some1",
        r#"{ "value": 11 }"#,
        ModificationTime::UnixTimestamp(2),
    );

    // Give the async closure some time to run since tests are executed
    // in single threaded environment.
    tokio::time::sleep(Duration::from_millis(SLEEP_TIME_MS)).await;
    // The flag_val should still not change since the notification for the config
    // change hasn't been fired.
    assert_eq!(flag_val.load(Ordering::Relaxed), 1);

    // Mark the config for refresh
    test_source.insert_to_refresh("some1".to_owned());
    // Ensure the updater thread has run
    thread::yield_now();
    thread::sleep(Duration::from_millis(SLEEP_TIME_MS));
    // Give the async closure some time to run since tests are executed
    // in single threaded environment.
    tokio::time::sleep(Duration::from_millis(SLEEP_TIME_MS)).await;

    // The flag_val should now be updated to refect two things.
    // 1. The loop in the async closure should have executed exactly once.
    // 2. The value fetched from the watcher is the latest value.
    assert_eq!(flag_val.load(Ordering::Relaxed), 11);
    task_handle.abort();
}

#[tokio::test]
async fn test_config_update_watcher_multiple_updates() {
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
    let watcher = handle1.watcher();
    let flag_val = Arc::new(AtomicI64::new(1));

    // Validate if fetching Config Update Watcher for dynamic sourced config works
    // successfully
    assert!(watcher.is_ok());
    let mut watcher = watcher.unwrap();

    let task_handle = tokio::spawn({
        let flag_val = flag_val.clone();
        async move {
            let mut iteration = 1;
            loop {
                let new_val = watcher.wait_for_next().await.unwrap().value * iteration;
                flag_val.store(new_val, Ordering::Relaxed);
                iteration += 1;
            }
        }
    });
    // Give the async closure some time to run since tests are executed
    // in single threaded environment.
    tokio::time::sleep(Duration::from_millis(SLEEP_TIME_MS)).await;
    // Ensure that the flag_val has not changed inspite of the watcher waiting
    // for the next configuration value. Given that the underlying config hasn't
    // changed, the watcher should be sleeping and thus flag_val should not be updated.
    assert_eq!(flag_val.load(Ordering::Relaxed), 1);

    // Update the config multiple times.
    test_source.insert_config(
        "some1",
        r#"{ "value": 11 }"#,
        ModificationTime::UnixTimestamp(2),
    );
    test_source.insert_config(
        "some1",
        r#"{ "value": 12 }"#,
        ModificationTime::UnixTimestamp(3),
    );
    test_source.insert_config(
        "some1",
        r#"{ "value": 13 }"#,
        ModificationTime::UnixTimestamp(4),
    );
    test_source.insert_config(
        "some1",
        r#"{ "value": 14 }"#,
        ModificationTime::UnixTimestamp(5),
    );

    // Give the async closure some time to run since tests are executed
    // in single threaded environment.
    tokio::time::sleep(Duration::from_millis(SLEEP_TIME_MS)).await;
    // The flag_val should still not change since the notification for the config
    // changes hasn't been fired.
    assert_eq!(flag_val.load(Ordering::Relaxed), 1);

    // Mark the config for refresh
    test_source.insert_to_refresh("some1".to_owned());
    // Ensure the updater thread has run
    thread::yield_now();
    thread::sleep(Duration::from_millis(SLEEP_TIME_MS));
    // Give the async closure some time to run since tests are executed
    // in single threaded environment.
    tokio::time::sleep(Duration::from_millis(SLEEP_TIME_MS)).await;

    // The flag_val should now be updated to refect two things.
    // 1. The loop in the async closure should have executed exactly once.
    // 2. The value fetched from the watcher is the last value that was
    // published by the config source. The intermediate values are not
    // accessible anymore
    assert_eq!(flag_val.load(Ordering::Relaxed), 14);
    task_handle.abort();
}

#[test]
fn test_config_update_watcher_for_static_configs() {
    let result = ConfigHandle::<TestConfig>::from_json(r#"{ "value": 44 }"#)
        .expect("failed to deserialize json");
    // Validate that fetching ConfigUpdateWatcher for statically sourced configs
    // is not supported and results in an error.
    assert!(result.watcher().is_err());
}
