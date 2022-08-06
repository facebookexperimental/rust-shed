/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
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

        use thiserror::Error;

        use crate::facet_impls::simple_one::SimpleOne;
        use crate::facets::one::ArcOne;

        #[derive(Debug, Eq, PartialEq, Error)]
        #[error("value must be one")]
        pub struct OneError;

        pub struct SimpleFactory;

        #[facet::factory(value: u32)]
        impl SimpleFactory {
            fn one(&self, value: &u32) -> Result<ArcOne, OneError> {
                if *value == 1 {
                    Ok(Arc::new(SimpleOne))
                } else {
                    Err(OneError)
                }
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

    let ok1 = factory.build::<containers::Basic>(1).unwrap();

    use crate::facets::one::OneRef;
    assert_eq!(ok1.one().get(), 1);

    use crate::factories::simple_factory::OneError;
    match factory.build::<containers::Basic>(2) {
        Err(facet::FactoryError::FacetBuildFailed { name, source }) => {
            assert_eq!(name, "one");
            assert_eq!(source.downcast::<OneError>().unwrap(), OneError);
        }
        _ => panic!("build with two should fail with facet build error"),
    }
}
