/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#[doc(hidden)]
pub mod macro_export {
    pub use frunk;
    pub trait ToTuple {
        type Output;
        fn to_tuple(self) -> Self::Output;
    }
}

// macro to implement: ToTuple for tuples and hlist of a given cardinality
macro_rules! impl_to_tuple_for {
    ($($seq:ident),*) => {
        impl<$($seq,)*> $crate::macro_export::ToTuple for ($($seq,)*) {
            type Output = ($($seq,)*);
            fn to_tuple(self) -> Self::Output {
                self
            }
        }

        impl<$($seq,)*> $crate::macro_export::ToTuple for $crate::macro_export::frunk::HList![$($seq),*] {
            type Output = ($($seq,)*);
            #[allow(non_snake_case)]
            fn to_tuple(self) -> ($($seq,)*) {
                match self {
                    $crate::macro_export::frunk::hlist_pat![$($seq),*] => ($($seq,)*)
                }
            }
        }
    }
}

// macro to implement: ToTuple for tuples and hlist of all cardinalities up-to a given cardinality
macro_rules! impl_to_tuple_up_to {
    ($head:ident) => {
        impl_to_tuple_for!($head);
    };
    ($head:ident, $($tail:ident),+) => {
        impl_to_tuple_for!($head, $($tail),*);
        impl_to_tuple_up_to!($($tail),*);
    };
}

// generate implementations up to length 40
impl_to_tuple_up_to!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39,
    T40
);

#[cfg(test)]
mod tests {
    use frunk::hlist;

    use crate::macro_export::ToTuple;

    #[test]
    fn test_convert() {
        let tup = ("foo", 42, true);
        assert_eq!(tup, tup.to_tuple());

        let hlist = hlist![tup.0, tup.1, tup.2];
        assert_eq!(tup, hlist.to_tuple());
    }
}
