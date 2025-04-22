/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Abomonable Strings
//! ==================
//!
//! Strings that can be serialized with abomonation and maintain a particular
//! alignment in the abomonation buffer.

use std::io::Result as IoResult;
use std::io::Write;

use abomonation::Abomonation;
use quickcheck_arbitrary_derive::Arbitrary;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Arbitrary)]
pub struct AbomonableString<const ALIGN: usize>(String);

impl<const ALIGN: usize> AbomonableString<ALIGN> {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl<const ALIGN: usize> From<String> for AbomonableString<ALIGN> {
    fn from(s: String) -> Self {
        Self(s)
    }
}

#[inline]
/// How many bytes of padding are needed to take us to the next alignment
/// boundary.
const fn remainder<const ALIGN: usize>(size: usize) -> usize {
    if size == 0 {
        0
    } else {
        ALIGN - ((size - 1) % ALIGN) - 1
    }
}

impl<const ALIGN: usize> std::ops::Deref for AbomonableString<ALIGN> {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// This implementation is based on the upstream implementation for String.
impl<const ALIGN: usize> Abomonation for AbomonableString<ALIGN> {
    #[inline]
    unsafe fn entomb<W: Write>(&self, write: &mut W) -> IoResult<()> {
        let pad_len = remainder::<ALIGN>(self.0.len());
        write.write_all(self.0.as_bytes())?;
        write.write_all(&[0; ALIGN][..pad_len])?;
        Ok(())
    }

    #[inline]
    unsafe fn exhume<'a, 'b>(&'a mut self, bytes: &'b mut [u8]) -> Option<&'b mut [u8]> {
        unsafe {
            let padded_len = self.0.len() + remainder::<ALIGN>(self.0.len());
            if padded_len > bytes.len() {
                None
            } else {
                let (mine, rest) = bytes.split_at_mut(padded_len);
                // SAFETY: we ensure the resulting string is read-only by only
                // allowing a shared reference to it once exhumed.
                std::ptr::write(
                    self,
                    Self(String::from_raw_parts(
                        mine.as_mut_ptr(),
                        self.0.len(),
                        self.0.len(),
                    )),
                );
                Some(rest)
            }
        }
    }

    #[inline]
    fn extent(&self) -> usize {
        self.0.len() + remainder::<ALIGN>(self.0.len())
    }
}

#[cfg(test)]
mod tests {
    use super::AbomonableString;

    fn align_n<const ALIGN: usize>() {
        for l in 0..250 {
            let s: AbomonableString<ALIGN> = "x".repeat(l).into();
            let mut v = Vec::new();

            // Abomonate the string
            unsafe {
                abomonation::encode(&s, &mut v).expect("Abomonate should succeed");
            }

            // The buffer length should match the alignment.
            assert!(v.len() % ALIGN == 0);

            // Extract it back out
            let (d, rem) =
                unsafe { abomonation::decode(&mut v).expect("Disabomonate should succeed") };

            // The string should match
            assert_eq!(&s, d);

            // We should have consumed the padding (if any)
            assert_eq!(rem.len(), 0);
        }
    }

    #[test]
    fn align_2() {
        align_n::<2>();
    }

    #[test]
    fn align_4() {
        align_n::<4>();
    }

    #[test]
    fn align_8() {
        align_n::<8>();
    }
}
