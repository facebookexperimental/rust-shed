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
        pub struct One;

        impl One {
            pub fn get(&self) -> u32 {
                1
            }
        }
    }

    pub mod two {
        #[facet::facet]
        #[derive(Clone)]
        pub enum Two {
            Value(u32),
            Constant,
        }

        impl Two {
            pub fn get(&self) -> u32 {
                match self {
                    Two::Value(n) => *n,
                    Two::Constant => 2,
                }
            }
        }
    }
}

pub mod factories {
    pub mod simple_factory {
        use crate::facets::one::ArcOne;
        use crate::facets::one::One;
        use crate::facets::two::ArcTwo;
        use crate::facets::two::Two;
        use std::sync::Arc;

        pub struct SimpleFactory;

        #[facet::factory(two_impl: Two)]
        impl SimpleFactory {
            fn one(&self) -> ArcOne {
                Arc::new(One)
            }

            fn two(&self, two_impl: &Two) -> ArcTwo {
                Arc::new(two_impl.clone())
            }
        }
    }
}

pub mod containers {
    use crate::facets::one::One;
    use crate::facets::two::Two;

    #[facet::container]
    pub struct Basic {
        #[facet]
        one: One,

        #[facet]
        two: Two,
    }
}

use crate::facets::one::OneRef;
use crate::facets::two::Two;
use crate::facets::two::TwoRef;

fn test_values(container: impl OneRef + TwoRef) {
    assert_eq!(container.one().get(), 1);
    assert_eq!(container.two().get(), 2);
}

#[test]
fn main() {
    let factory = factories::simple_factory::SimpleFactory;

    let value = factory.build::<containers::Basic>(Two::Value(2)).unwrap();
    test_values(&value);

    let constant = factory.build::<containers::Basic>(Two::Constant).unwrap();
    test_values(&constant);
}
