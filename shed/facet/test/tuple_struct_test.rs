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

    pub mod two {
        #[facet::facet]
        pub struct Two;

        impl Two {
            pub fn get(&self) -> u32 {
                2
            }
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
        use crate::facets::two::ArcTwo;
        use crate::facets::two::Two;

        pub struct SimpleFactory;

        #[facet::factory()]
        impl SimpleFactory {
            fn one(&self) -> ArcOne {
                Arc::new(SimpleOne)
            }

            fn two(&self) -> ArcTwo {
                Arc::new(Two)
            }
        }
    }
}

pub mod containers {
    use crate::facets::one::One;
    use crate::facets::two::Two;

    #[facet::container]
    pub struct TupleStruct(dyn One, Two);

    #[facet::container]
    pub struct NormalStruct {
        #[facet]
        pub one: dyn One,

        #[facet]
        two: Two,
    }

    #[facet::container]
    pub struct DelegateStruct {
        #[facet]
        pub one: dyn One,

        #[delegate(Two)]
        pub normal_struct: NormalStruct,
    }
}

fn check_ts_like(ts: &impl crate::containers::TupleStructLike) {
    use crate::facets::one::OneRef;
    use crate::facets::two::TwoRef;

    assert_eq!(ts.one().get(), 1);
    assert_eq!(ts.two().get(), 2);
}

fn check_ns_like(ns: &impl crate::containers::NormalStructLike) {
    use crate::facets::one::OneRef;
    use crate::facets::two::TwoRef;

    assert_eq!(ns.one().get(), 1);
    assert_eq!(ns.two().get(), 2);
}

fn check_ds_like(ds: &impl crate::containers::DelegateStructLike) {
    use crate::facets::one::OneRef;
    use crate::facets::two::TwoRef;

    assert_eq!(ds.one().get(), 1);
    assert_eq!(ds.two().get(), 2);
}

#[test]
fn main() {
    let factory = factories::simple_factory::SimpleFactory;

    let ts = factory.build::<containers::TupleStruct>().unwrap();

    use crate::facets::one::OneRef;
    use crate::facets::two::TwoRef;
    assert_eq!(ts.one().get(), 1);
    assert_eq!(ts.two().get(), 2);

    let ns = factory.build::<containers::NormalStruct>().unwrap();
    let ds = factory.build::<containers::DelegateStruct>().unwrap();

    check_ts_like(&ts);
    check_ts_like(&ns);
    check_ts_like(&ds);

    check_ns_like(&ts);
    check_ns_like(&ns);
    check_ns_like(&ds);

    check_ds_like(&ts);
    check_ds_like(&ns);
    check_ds_like(&ds);

    // This is a test and these are from the same compilation unit, so the
    // vtable address ought to match.
    #[allow(clippy::vtable_address_comparisons)]
    {
        // Note that `DelegateStruct` has two `dyn One` facets, but they are the
        // same object.
        use std::sync::Arc;
        assert_eq!(Arc::strong_count(&ds.one), 2);
        assert!(Arc::ptr_eq(&ds.one, &ds.normal_struct.one));
    }
}
