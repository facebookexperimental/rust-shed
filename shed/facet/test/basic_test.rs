/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

pub mod facets {
    pub mod one {
        #[facet::facet]
        pub trait One {
            fn get(&self) -> u32;
        }
    }
}

pub mod facet_impls {
    pub mod simple_one {
        use crate::facets::one::One;

        pub struct SimpleOne;

        impl One for SimpleOne {
            fn get(&self) -> u32 {
                1
            }
        }
    }
}

pub mod factories {
    pub mod simple_factory {
        use std::sync::Arc;

        use crate::facet_impls::simple_one::SimpleOne;
        use crate::facets::one::ArcOne;

        pub struct SimpleFactory;

        #[facet::factory()]
        impl SimpleFactory {
            fn one(&self) -> ArcOne {
                Arc::new(SimpleOne)
            }
        }
    }
}

pub mod containers {
    use crate::facets::one::One;

    #[facet::container]
    pub struct Basic {
        #[facet]
        one: dyn One,
    }
}

#[test]
fn main() {
    let factory = factories::simple_factory::SimpleFactory;

    let basic = factory.build::<containers::Basic>().unwrap();

    use crate::facets::one::OneRef;
    assert_eq!(basic.one().get(), 1);
}
