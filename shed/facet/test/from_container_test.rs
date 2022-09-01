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
        pub trait Two {
            fn get(&self) -> u32;
        }

        pub struct ConcreteTwo;
        impl Two for ConcreteTwo {
            fn get(&self) -> u32 {
                2
            }
        }
    }
}

pub mod factories {
    pub mod simple_factory {
        use std::sync::Arc;

        use crate::facets::one::ArcOne;
        use crate::facets::one::One;
        use crate::facets::two::ArcTwo;
        use crate::facets::two::ConcreteTwo;

        pub struct SimpleFactory;

        #[facet::factory]
        impl SimpleFactory {
            fn one(&self) -> ArcOne {
                Arc::new(One)
            }

            fn two(&self) -> ArcTwo {
                Arc::new(ConcreteTwo)
            }
        }
    }
}

#[macro_use]
pub mod containers {
    use crate::facets::one::One;
    use crate::facets::two::Two;
    use crate::facets::two::TwoArc;

    #[facet::container]
    pub struct Basic {
        #[facet]
        one: One,

        #[facet]
        two: dyn Two,
    }

    #[facet::container]
    pub struct JustTwo {
        #[facet]
        second: dyn Two,
    }

    pub fn to_just_two(other: impl TwoArc) -> JustTwo {
        just_two_from_container!(other)
    }
}

use crate::facets::one::OneRef;
use crate::facets::two::TwoRef;

fn test_values(container: impl OneRef + TwoRef) {
    assert_eq!(container.one().get(), 1);
    assert_eq!(container.two().get(), 2);
}

#[test]
fn main() {
    let factory = factories::simple_factory::SimpleFactory;

    let value = factory.build::<containers::Basic>().unwrap();
    test_values(&value);

    let just_two = containers::to_just_two(&value);
    assert_eq!(just_two.two().get(), 2);
}
