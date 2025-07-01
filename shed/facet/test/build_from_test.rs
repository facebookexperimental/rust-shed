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
    use crate::facets::two::TwoRef;

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

    #[facet::container]
    pub struct DelegatedJustTwo {
        #[init(just_two.two().get() == 2)]
        pub is_two: bool,

        #[delegate(dyn Two)]
        just_two: JustTwo,
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

    let just_two = containers::JustTwo::build_from(&value);
    assert_eq!(just_two.two().get(), 2);

    let delegated_just_two = containers::DelegatedJustTwo::build_from(&value);
    assert_eq!(delegated_just_two.two().get(), 2);
    assert!(delegated_just_two.is_two);

    let from_delegated_just_two = containers::JustTwo::build_from(&delegated_just_two);
    assert_eq!(from_delegated_just_two.two().get(), 2);
}
