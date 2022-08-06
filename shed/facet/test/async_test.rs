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
        #[async_trait::async_trait]
        pub trait One {
            async fn get(&self) -> u32;
        }
    }

    pub mod two {
        #[facet::facet]
        #[async_trait::async_trait]
        pub trait Two {
            async fn get(&self) -> u32;
        }
    }

    pub mod zero {
        #[facet::facet]
        pub trait Zero {
            fn get(&self) -> u32;
        }
    }
}

pub mod facet_impls {
    pub mod async_one {
        use crate::facets::one::One;

        pub struct AsyncOne;

        #[async_trait::async_trait]
        impl One for AsyncOne {
            async fn get(&self) -> u32 {
                1
            }
        }
    }

    pub mod async_two {
        use crate::facets::one::ArcOne;
        use crate::facets::two::Two;
        use crate::facets::zero::ArcZero;

        pub struct AsyncTwo(pub ArcZero, pub ArcOne);

        #[async_trait::async_trait]
        impl Two for AsyncTwo {
            async fn get(&self) -> u32 {
                let a = self.1.get().await;
                let b = self.0.get();
                let c = self.1.get().await;
                a + b + c
            }
        }
    }

    pub mod sync_zero {
        use crate::facets::zero::Zero;

        pub struct SyncZero;

        impl Zero for SyncZero {
            fn get(&self) -> u32 {
                0
            }
        }
    }
}

pub mod factories {
    pub mod async_factory {
        use std::sync::Arc;

        use crate::facet_impls::async_one::AsyncOne;
        use crate::facet_impls::async_two::AsyncTwo;
        use crate::facet_impls::sync_zero::SyncZero;
        use crate::facets::one::ArcOne;
        use crate::facets::two::ArcTwo;
        use crate::facets::zero::ArcZero;

        pub struct AsyncFactory;

        #[facet::factory]
        impl AsyncFactory {
            async fn one(&self) -> ArcOne {
                Arc::new(AsyncOne)
            }

            async fn two(&self, zero: &ArcZero, one: &ArcOne) -> ArcTwo {
                Arc::new(AsyncTwo(zero.clone(), one.clone()))
            }

            async fn zero(&self) -> ArcZero {
                Arc::new(SyncZero)
            }
        }
    }
}

pub mod containers {
    use std::sync::Arc;

    use crate::facets::one::One;
    use crate::facets::two::Two;
    use crate::facets::zero::Zero;

    #[facet::container]
    pub struct Basic {
        #[facet]
        one: dyn One,

        #[facet]
        two: dyn Two,
    }

    #[facet::container]
    pub struct InnerOne {
        #[facet]
        one: dyn One,
    }

    #[facet::container]
    pub struct InnerTwo {
        #[facet]
        two: dyn Two,

        #[facet]
        zero: dyn Zero,
    }

    #[facet::container]
    pub struct Outer {
        #[delegate(dyn One)]
        inner1: InnerOne,

        #[delegate(dyn Zero, dyn Two)]
        inner2: Arc<InnerTwo>,
    }
}

#[tokio::test]
async fn main() {
    let factory = factories::async_factory::AsyncFactory;

    let basic = factory.build::<containers::Basic>().await.unwrap();

    use crate::facets::one::OneRef;
    use crate::facets::two::TwoRef;
    use crate::facets::zero::ZeroRef;
    assert_eq!(basic.one().get().await, 1);
    assert_eq!(basic.two().get().await, 2);

    let outer = factory.build::<containers::Outer>().await.unwrap();
    assert_eq!(outer.zero().get(), 0);
    assert_eq!(outer.one().get().await, 1);
    assert_eq!(outer.two().get().await, 2);
}
