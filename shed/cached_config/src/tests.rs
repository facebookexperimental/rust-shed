/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::{anyhow, Result};
use serde_derive::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{ConfigHandle, ConfigStore, Entity, Source};

#[derive(Debug)]
struct TestSource {
    path_to_config: Arc<Mutex<HashMap<String, Entity>>>,
    to_refresh: Arc<Mutex<HashSet<String>>>,
}

impl Source for TestSource {
    fn config_for_path(&self, path: &str) -> Result<Entity> {
        self.path_to_config
            .lock()
            .expect("poisoned lock")
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow!("Config not present for {:?}", path))
    }

    fn paths_to_refresh<'a>(&self, paths: &mut dyn Iterator<Item = &'a str>) -> Vec<&'a str> {
        let to_refresh = self.to_refresh.lock().expect("poisoned lock");
        paths.filter(|p| to_refresh.contains(*p)).collect()
    }
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct TestConfig {
    value: i64,
}

fn insert_test(map: &mut HashMap<String, Entity>, key: &str, contents: &str, mod_time: u64) {
    map.insert(
        key.to_owned(),
        Entity {
            contents: contents.to_owned(),
            mod_time,
            version: None,
        },
    );
}

fn get_test_handle(store: &ConfigStore, path: &str) -> Result<ConfigHandle<TestConfig>> {
    store.get_config_handle(path.to_owned())
}

#[test]
fn test_config_store() {
    let path_to_config = Arc::new(Mutex::new(HashMap::new()));
    let to_refresh = Arc::new(Mutex::new(HashSet::new()));

    {
        let mut path_to_config = path_to_config.lock().expect("poisoned lock");
        insert_test(&mut path_to_config, "some1", r#"{ "value": 1 }"#, 1);
        insert_test(&mut path_to_config, "some2", r#"{ "value": 2 }"#, 1);
    }

    let store = {
        let test_source = TestSource {
            path_to_config: path_to_config.clone(),
            to_refresh: to_refresh.clone(),
        };

        ConfigStore::new(Arc::new(test_source), Duration::from_millis(2), None)
    };

    let handle1 = get_test_handle(&store, "some1").expect("Failed to get handle1");
    let handle2 = get_test_handle(&store, "some2").expect("Failed to get handle2");

    assert_eq!(*handle1.get(), TestConfig { value: 1 });
    assert_eq!(*handle2.get(), TestConfig { value: 2 });
    assert!(
        get_test_handle(&store, "some4").is_err(),
        "some4 should have not exist"
    );

    {
        let mut path_to_config = path_to_config.lock().expect("poisoned lock");
        insert_test(&mut path_to_config, "some1", r#"{ "value": 11 }"#, 1);
        insert_test(&mut path_to_config, "some2", r#"{ "value": 22 }"#, 2);
        insert_test(&mut path_to_config, "some4", r#"{ "value": 4 }"#, 1);
    }

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

    {
        let mut to_refresh = to_refresh.lock().expect("poisoned lock");
        to_refresh.insert("some1".to_owned());
        to_refresh.insert("some2".to_owned());
    }

    // Ensure the updater thread has run
    thread::yield_now();
    thread::sleep(Duration::from_secs(1));

    // handle1 remains the same, because the mod_time has not changed
    assert_eq!(*handle1.get(), TestConfig { value: 1 });
    assert_eq!(*handle2.get(), TestConfig { value: 22 });
}

#[test]
fn test_config_handle_from_json() {
    let result = ConfigHandle::<TestConfig>::from_json(r#"{ "value": 44 }"#)
        .expect("failed to deserialize json")
        .get();
    assert_eq!(*result, TestConfig { value: 44 });
}
