/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

/// Helper for generating keys in the correct format
#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct KeyGen {
    category: String,
    codever: u32,
    sitever: u32,
}

impl KeyGen {
    /// Construct a new KeyGen helper. The category must be pure ascii
    pub fn new<C>(category: C, codever: u32, sitever: u32) -> Self
    where
        C: AsRef<str>,
    {
        let category = category.as_ref();
        let all_ascii = category.is_ascii();
        let no_bad = !category.chars().any(|c| c == ' ' || c == ':');

        assert!(all_ascii, "category is not pure ascii");
        assert!(no_bad, "category contains invalid characters");

        KeyGen {
            category: category.to_string(),
            codever,
            sitever,
        }
    }

    /// Construct a new key with the given `id`
    pub fn key<ID>(&self, id: ID) -> String
    where
        ID: AsRef<str>,
    {
        let id = id.as_ref();
        let is_ascii = id.is_ascii();
        let no_space = !id.chars().any(|c| c == ' ');

        assert!(is_ascii, "id \"{id}\" is not pure ascii");
        assert!(no_space, "id \"{id}\" contains spaces");

        format!(
            "{}:{}:c{}:s{}",
            self.category, id, self.codever, self.sitever
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn keygen() {
        let kg = KeyGen::new("test.prefix", 1, 2);
        let k = kg.key("foo");

        assert_eq!(k, "test.prefix:foo:c1:s2");
    }

    #[test]
    #[should_panic(expected = "is not pure ascii")]
    fn keygen_pfx_nonascii() {
        let _ = KeyGen::new("L\u{00F6}we\u{8001}\u{864E}L\u{00E9}opard", 1, 2);
    }

    #[test]
    #[should_panic(expected = "contains invalid characters")]
    fn keygen_pfx_space() {
        let _ = KeyGen::new("foo bar", 1, 2);
    }

    #[test]
    #[should_panic(expected = "is not pure ascii")]
    fn keygen_nonascii() {
        let kg = KeyGen::new("test.prefix", 1, 2);
        let _ = kg.key("L\u{00F6}we\u{8001}\u{864E}L\u{00E9}opard"); // "Löwe老虎Léopard"
    }

    #[test]
    #[should_panic(expected = "contains spaces")]
    fn keygen_space() {
        let kg = KeyGen::new("test.prefix", 1, 2);
        let _ = kg.key("foo bar");
    }
}
