/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![feature(unboxed_closures)]
#![feature(fn_traits)]

#[doc(hidden)]
pub mod macro_export {
    pub use frunk;
    pub struct ToTuple;
    pub trait ConvertToTuple {
        type Output;
        fn convert(self) -> Self::Output;
    }
}

// macro to implement: ToTuple(hlist![…]) => (…,)
macro_rules! impl_to_tuple_for_seq {
    ($($seq:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($seq,)*> FnOnce<($crate::macro_export::frunk::HList![$($seq),*],)> for $crate::macro_export::ToTuple {
            type Output = ($($seq,)*);
            #[inline]
            extern "rust-call" fn call_once(self, (this,): ($crate::macro_export::frunk::HList![$($seq),*],)) -> Self::Output {
                match this {
                    $crate::macro_export::frunk::hlist_pat![$($seq),*] => ($($seq,)*)
                }
            }
        }

        impl<$($seq,)*> $crate::macro_export::ConvertToTuple for ($($seq,)*) {
            type Output = ($($seq,)*);
            fn convert(self) -> Self::Output {
                self
            }
        }

        impl<$($seq,)*> $crate::macro_export::ConvertToTuple for $crate::macro_export::frunk::HList![$($seq),*] {
            type Output = ($($seq,)*);
            fn convert(self) -> ($($seq,)*) {
                $crate::macro_export::ToTuple(self)
            }
        }
    }
}

// generate implementations up to length 40
impl_to_tuple_for_seq!(T0);
impl_to_tuple_for_seq!(T0, T1);
impl_to_tuple_for_seq!(T0, T1, T2);
impl_to_tuple_for_seq!(T0, T1, T2, T3);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4, T5);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4, T5, T6);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4, T5, T6, T7);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4, T5, T6, T7, T8);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_to_tuple_for_seq!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39
);
impl_to_tuple_for_seq!(
    T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16, T17, T18, T19, T20,
    T21, T22, T23, T24, T25, T26, T27, T28, T29, T30, T31, T32, T33, T34, T35, T36, T37, T38, T39,
    T40
);

#[cfg(test)]
mod tests {
    use frunk::hlist;

    use crate::macro_export::ConvertToTuple;
    use crate::macro_export::ToTuple;

    #[test]
    fn hlist_into_tuple() {
        let tup = ("foo", 42, true);
        let hlist = hlist![tup.0, tup.1, tup.2];
        assert_eq!(tup, ToTuple(hlist));
    }

    #[test]
    fn test_convert() {
        let tup = ("foo", 42, true);
        assert_eq!(tup, tup.convert());

        let hlist = hlist![tup.0, tup.1, tup.2];
        assert_eq!(tup, hlist.convert());
    }
}
