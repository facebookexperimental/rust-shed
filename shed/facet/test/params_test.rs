/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

pub mod facets {
    pub mod value {
        #[facet::facet]
        pub trait Value {
            fn get(&self) -> u32;
        }
    }

    pub mod name {
        #[facet::facet]
        pub trait Name {
            fn obtain(&self) -> &str;
        }
    }
}

pub mod facet_impls {
    pub mod simple_value {
        use crate::facets::value::Value;

        pub struct SimpleValue(pub u32);

        impl Value for SimpleValue {
            fn get(&self) -> u32 {
                self.0
            }
        }
    }

    pub mod simple_name {
        use crate::facets::name::Name;

        pub struct SimpleName(pub String);

        impl Name for SimpleName {
            fn obtain(&self) -> &str {
                self.0.as_str()
            }
        }
    }
}

pub mod factories {
    pub mod simple_factory {
        use std::sync::Arc;

        use crate::facet_impls::simple_name::SimpleName;
        use crate::facet_impls::simple_value::SimpleValue;
        use crate::facets::name::ArcName;
        use crate::facets::value::ArcValue;

        pub struct SimpleFactory;

        #[facet::factory(name_param: String, init_value: u32)]
        impl SimpleFactory {
            fn value(&self, init_value: &u32) -> ArcValue {
                Arc::new(SimpleValue(*init_value))
            }

            fn name(&self, name_param: &str) -> ArcName {
                Arc::new(SimpleName(name_param.to_string()))
            }
        }
    }
}

pub mod containers {
    use crate::facets::name::Name;
    use crate::facets::value::Value;

    #[facet::container]
    pub struct Parameterised {
        #[facet]
        name: dyn Name,

        #[facet]
        value: dyn Value,

        #[init(format!("{}({})", name.obtain(), value.get()))]
        pub debug: String,
    }
}

use containers::Parameterised;
use facets::name::NameRef;
use facets::value::ValueRef;

fn check_item(item: impl NameRef + ValueRef + Copy, name: &str, value: u32) {
    assert_eq!(item.name().obtain(), name);
    assert_eq!(item.value().get(), value);
}

#[test]
fn main() {
    let factory = factories::simple_factory::SimpleFactory;

    let param1 = factory
        .build::<Parameterised>(String::from("first"), 1)
        .unwrap();
    let param2 = factory
        .build::<Parameterised>(String::from("second"), 2)
        .unwrap();

    check_item(&param1, "first", 1);
    check_item(&param2, "second", 2);
    assert_eq!(param1.debug, "first(1)");
    assert_eq!(param2.debug, "second(2)");
}
