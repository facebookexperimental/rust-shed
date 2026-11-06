/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use crate::Binary;
use crate::QueryParameter;
use crate::mysql::AsSql;
use crate::mysql_async::Value;
use crate::mysql_async::prelude::ToValue;

fn assert_existing<T: ToValue>(value: &T) {
    assert_eq!(
        QueryParameter::to_sqlite_value(value),
        ToValue::to_value(value)
    );
    for no_backslash_escape in [false, true] {
        let expected = ToValue::to_value(&value).as_sql(no_backslash_escape);
        assert_eq!(AsSql::as_sql(value, no_backslash_escape), expected);
        assert_eq!(AsSql::as_sql(&value, no_backslash_escape), expected);
    }
}

fn scalar_cases() -> Vec<Value> {
    vec![
        Value::NULL,
        Value::Int(i64::MIN),
        Value::Int(-1),
        Value::Int(0),
        Value::Int(i64::MAX),
        Value::UInt(u64::MAX),
        Value::Float(-0.0),
        Value::Float(f32::MIN),
        Value::Float(f32::MAX),
        Value::Double(-0.0),
        Value::Double(f64::MIN_POSITIVE),
        Value::Double(f64::MIN),
        Value::Double(f64::MAX),
        Value::Date(0, 0, 0, 0, 0, 0, 0),
        Value::Date(2000, 2, 29, 0, 0, 0, 0),
        Value::Date(2026, 11, 6, 12, 34, 56, 0),
        Value::Date(9999, 12, 31, 23, 59, 59, 999_999),
        Value::Time(false, 0, 0, 0, 0, 0),
        Value::Time(true, 1, 2, 3, 4, 0),
        Value::Time(false, 1, 2, 3, 4, 567_890),
        Value::Time(true, 34, 22, 59, 59, 999_999),
    ]
}

fn byte_cases() -> Vec<Vec<u8>> {
    let mut cases = vec![
        vec![],
        b"\0'\"\\\n\r\x1a; -- /* */".to_vec(),
        "aé\u{7ff}\u{800}\u{ffff}\u{10000}\u{10ffff}"
            .as_bytes()
            .to_vec(),
        vec![0xc0, 0x80],
        vec![0xed, 0xa0, 0x80],
        vec![0xf4, 0x90, 0x80, 0x80],
        vec![0xe2, 0x82],
    ];
    cases.extend((0..=255).map(|byte| vec![byte]));
    for len in [255, 256, 4095, 4096, 65_536, 1_048_576] {
        cases.push((0..=255).cycle().take(len).collect());
    }
    cases.push(vec![0; 1_048_576]);
    cases
}

#[test]
fn existing_parameter_encodings_match() {
    struct Custom(Value);
    impl ToValue for Custom {
        fn to_value(&self) -> Value {
            self.0.clone()
        }
    }

    for value in scalar_cases() {
        assert_existing(&value);
        assert_existing(&Custom(value));
    }
    assert_existing(&true);
    assert_existing(&false);
    assert_existing(&Some(42u64));
    assert_existing(&None::<u64>);
    assert_existing(&"'\\\0é");
    assert_existing(&"'\\\0é".to_owned());
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(AsSql::as_sql(&value, false), value.to_value().as_sql(false));
    }
}

#[test]
fn existing_byte_encodings_match() {
    for bytes in byte_cases()
        .into_iter()
        .chain((0..=255).flat_map(|first| (0..=255).map(move |second| vec![first, second])))
    {
        assert_existing(&bytes);
        assert_existing(&bytes.as_slice());
    }
    let unicode: String = (0..=0x10ffff).filter_map(char::from_u32).collect();
    assert_existing(&unicode);
}

#[test]
fn binary_bytes_round_trip_sqlite() {
    let conn = crate::rusqlite::Connection::open_in_memory().unwrap();
    for bytes in byte_cases()
        .into_iter()
        .chain((0..=255).flat_map(|first| (0..=255).map(move |second| vec![first, second])))
    {
        let literal = Binary(&bytes).as_sql(false);
        let actual: Vec<u8> = conn
            .query_row(&format!("SELECT {literal}"), [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, bytes);
        assert_eq!(
            QueryParameter::to_sqlite_value(&Binary(&bytes)),
            bytes.to_value()
        );
    }
}

#[allow(dead_code, unused_imports)]
mod read {
    crate::_read_query_impl!((value: Value, >list values: Value) -> (Vec<u8>) {
        mysql("SELECT {value} WHERE {value} IN {values}")
        sqlite("SELECT {value} WHERE {value} IN {values}")
    });

    #[test]
    fn generated_sql_matches() {
        let values: Vec<_> = super::scalar_cases()
            .into_iter()
            .chain(super::byte_cases().into_iter().map(Value::Bytes))
            .collect();
        for value in &values {
            for list in [&values[..0], &values[..1], &values[..24]] {
                let legacy_list = list
                    .iter()
                    .map(|v| ToValue::to_value(&v).as_sql(false))
                    .collect::<Vec<_>>()
                    .join(", ");
                let legacy_value = ToValue::to_value(&value).as_sql(false);
                assert_eq!(
                    mysql_query(value, list),
                    format!("SELECT {legacy_value} WHERE {legacy_value} IN ({legacy_list})")
                );
            }
        }
    }
}

#[allow(dead_code, unused_imports)]
mod write {
    crate::_write_query_impl!((value: Value, >list values: Value) {
        insert_or_ignore,
        mysql("{insert_or_ignore} INTO t SELECT {value} WHERE {value} IN {values}")
        sqlite("{insert_or_ignore} INTO t SELECT {value} WHERE {value} IN {values}")
    });

    #[test]
    fn generated_sql_matches() {
        for value in super::scalar_cases()
            .into_iter()
            .chain(super::byte_cases().into_iter().map(Value::Bytes))
        {
            let literal = ToValue::to_value(&&value).as_sql(false);
            assert_eq!(
                mysql_query(&value, std::slice::from_ref(&value)),
                format!("INSERT IGNORE INTO t SELECT {literal} WHERE {literal} IN ({literal})")
            );
        }
    }
}

#[allow(dead_code, unused_imports)]
mod bulk {
    crate::_write_query_impl!(values: (id: u64, label: str, value: [u8]), (extra: Value) {
        none,
        mysql("INSERT INTO t VALUES {values} ON DUPLICATE KEY UPDATE extra = {extra}")
        sqlite("INSERT INTO t VALUES {values} ON CONFLICT DO UPDATE SET extra = {extra}")
    });

    #[test]
    fn generated_sql_matches() {
        for bytes in super::byte_cases() {
            let value = bytes.as_slice();
            let values = &[(&1, "'\\\0é", value), (&u64::MAX, "", value)];
            let mut legacy_rows = Vec::new();
            for row in values {
                let (id, label, value) = row;
                legacy_rows.push(format!(
                    "({}, {}, {})",
                    id.to_value().as_sql(false),
                    label.to_value().as_sql(false),
                    value.to_value().as_sql(false)
                ));
            }
            let extra = Value::Bytes(b"'\\\0".to_vec());
            assert_eq!(
                mysql_query(values, &extra),
                format!(
                    "INSERT INTO t VALUES {} ON DUPLICATE KEY UPDATE extra = {}",
                    legacy_rows.join(", "),
                    ToValue::to_value(&&extra).as_sql(false)
                )
            );
        }
    }
}

#[cfg(shed_sql_mysql)]
#[allow(unused_imports)]
mod mysql {
    use fbinit::FacebookInit;
    use mysql_client::InstanceRequirement;
    use sql_tests_lib::mysql_test_lib::setup_mysql_test_connection;

    use super::*;

    crate::queries! {
        write Create() { none, "CREATE TEMPORARY TABLE binary_encoding_test (id INT PRIMARY KEY, old_value LONGBLOB, new_value LONGBLOB)" }
        write Insert(values: (id: i64, old: &[u8], new: Binary<'_>)) {
            none, "INSERT INTO binary_encoding_test VALUES {values}"
        }
        read Read() -> (i64, Vec<u8>, Vec<u8>) {
            "SELECT id, old_value, new_value FROM binary_encoding_test ORDER BY id"
        }
        write Drop() { none, "DROP TEMPORARY TABLE binary_encoding_test" }
    }

    #[fbinit::test]
    async fn binary_bytes_round_trip_mysql(fb: FacebookInit) -> anyhow::Result<()> {
        let conn = setup_mysql_test_connection(fb, None, InstanceRequirement::Master).await?;
        let transaction = conn.start_transaction().await?;
        let (mut transaction, _) = Create::query_with_transaction(transaction).await?;
        let inputs = byte_cases();
        let records: Vec<_> = inputs
            .iter()
            .enumerate()
            .map(|(id, bytes)| (id as i64, bytes.as_slice(), Binary(bytes)))
            .collect();
        let values: Vec<_> = records
            .iter()
            .map(|(id, old, new)| (id, old, new))
            .collect();
        for batch in values.chunks(128) {
            (transaction, _) = Insert::query_with_transaction(transaction, batch).await?;
        }
        let (transaction, rows) = Read::query_with_transaction(transaction).await?;
        assert_eq!(rows.len(), inputs.len());
        for (id, old, new) in rows {
            assert_eq!(old, inputs[id as usize]);
            assert_eq!(new, old);
        }
        let (transaction, _) = Drop::query_with_transaction(transaction).await?;
        transaction.commit().await?;
        Ok(())
    }
}
