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

    pub mod two {
        #[facet::facet]
        pub trait Two {
            fn get(&self) -> u32;
        }
    }

    pub mod three {
        #[facet::facet]
        pub trait Three {
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

    pub mod combined_two {
        use crate::facets::one::ArcOne;
        use crate::facets::two::Two;

        pub struct CombinedTwo(pub ArcOne);

        impl Two for CombinedTwo {
            fn get(&self) -> u32 {
                self.0.get() + self.0.get()
            }
        }
    }

    pub mod simple_three {
        use crate::facets::three::Three;

        pub struct SimpleThree;

        impl Three for SimpleThree {
            fn get(&self) -> u32 {
                // Special implementation of the `Three` trait which doesn't
                // use dependencies (e.g. a test double).  To distinguish this
                // version, we return three threes.
                333
            }
        }
    }

    pub mod combined_three {
        use crate::facets::one::ArcOne;
        use crate::facets::three::Three;
        use crate::facets::two::ArcTwo;

        pub struct CombinedThree(pub ArcOne, pub ArcTwo);

        impl Three for CombinedThree {
            fn get(&self) -> u32 {
                self.0.get() + self.1.get()
            }
        }
    }
}

pub mod factories {
    pub mod deps_factory {
        use std::sync::Arc;

        use crate::facet_impls::combined_three::CombinedThree;
        use crate::facet_impls::combined_two::CombinedTwo;
        use crate::facet_impls::simple_one::SimpleOne;
        use crate::facets::one::ArcOne;
        use crate::facets::three::ArcThree;
        use crate::facets::two::ArcTwo;

        pub struct DepsFactory;

        #[facet::factory()]
        impl DepsFactory {
            fn one(&self) -> ArcOne {
                Arc::new(SimpleOne)
            }

            fn two(&self, one: &ArcOne) -> ArcTwo {
                Arc::new(CombinedTwo(one.clone()))
            }

            fn three(&self, one: &ArcOne, two: &ArcTwo) -> ArcThree {
                Arc::new(CombinedThree(one.clone(), two.clone()))
            }
        }
    }

    pub mod just3_factory {
        use std::sync::Arc;

        use crate::facet_impls::simple_three::SimpleThree;
        use crate::facets::three::ArcThree;

        pub struct Just3Factory;

        #[facet::factory()]
        impl Just3Factory {
            fn three(&self) -> ArcThree {
                Arc::new(SimpleThree)
            }
        }
    }
}

pub mod containers {
    use crate::facets::one::One;
    use crate::facets::three::Three;
    use crate::facets::two::Two;

    #[facet::container]
    pub struct Deps {
        #[facet]
        one: dyn One,

        #[facet]
        two: dyn Two,

        #[facet]
        three: dyn Three,
    }

    #[facet::container]
    pub struct TwoOnly {
        #[facet]
        two: dyn Two,
    }

    #[facet::container]
    pub struct ThreeOnly {
        #[facet]
        three: dyn Three,
    }
}

use containers::Deps;
use containers::ThreeOnly;
use containers::TwoOnly;
use facets::one::OneRef;
use facets::three::ThreeRef;
use facets::two::TwoRef;

#[test]
fn deps_factory() {
    let factory = factories::deps_factory::DepsFactory;

    let deps = factory.build::<Deps>().unwrap();

    assert_eq!(deps.one().get(), 1);
    assert_eq!(deps.two().get(), 2);
    assert_eq!(deps.three().get(), 3);

    // DepsFactory can build `TwoOnly`, even though a subset of the facets are
    // needed.  The dependent `One` will be created.  No `Three`
    // implementation will be created.
    let two_only = factory.build::<TwoOnly>().unwrap();
    assert_eq!(two_only.two().get(), 2);

    // DepsFactory can build `ThreeOnly`, even though a subset of the facets
    // are needed.  The dependent `One` and `Two` implementations will also be
    // created.
    let three_only = factory.build::<ThreeOnly>().unwrap();

    assert_eq!(three_only.three().get(), 3);
}

#[test]
fn just_three_factory() {
    let factory = factories::just3_factory::Just3Factory;

    // This factory cannot build `Deps` or `TwoOnly` as it doesn't know how to
    // build `One` or `Two`.

    let three_only = factory.build::<ThreeOnly>().unwrap();

    assert_eq!(three_only.three().get(), 333);
}
