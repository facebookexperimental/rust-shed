/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

//! See examples for what code you can write with borrowed macro.
//!
//! # Examples
//!
//! ```
//! # use borrowed::borrowed;
//! struct A {
//!     x: u32,
//!     y: u32,
//!     z: u32,
//! }
//! impl A {
//!     fn foo(&self) {
//!         borrowed!(self.x, self.y, self.z);
//!         (move || {
//!             println!("{} {} {}", x, y, z);
//!         })();
//!     }
//! }
//! # fn main () {}
//! ```
//!
//! It also supports setting the borrow type if its ambiguous:
//! ```
//! # use borrowed::borrowed;
//! # fn main () {
//! let foo = "foo".to_string();
//! borrowed!(foo as bar: &str);
//! assert!(&foo == bar);
//! # }
//! ```
//!
//! It also supports setting a local alias:
//! ```
//! # use borrowed::borrowed;
//! # fn main () {
//! let foo = 42;
//! borrowed!(foo as bar);
//! assert!(&foo == bar);
//! # }
//! ```
//!
//! And the two can be combined:
//! ```
//! # use borrowed::borrowed;
//! # fn main () {
//! let foo = "foo".to_string();
//! borrowed!(foo as bar: &str);
//! assert!(&foo == bar);
//! # }
//! ```

/// See crate's documentation
#[macro_export]
macro_rules! borrowed {
    // Ambigous, so need to specify type
    ($i:ident as $alias:ident : $borrow:ty) => {
        let $alias: $borrow = {
            use std::borrow::Borrow;
            $i.borrow()
        };
    };
    // If borrow is unambigous
    ($i:ident as $alias:ident) => {
        let $alias = {
            use std::borrow::Borrow;
            $i.borrow()
        };
    };
    (mut $i:ident as $alias:ident : $borrow:ty) => {
        let mut $alias: $borrow = {
            use std::borrow::Borrow;
            $i.borrow()
        };
    };
    (mut $i:ident as $alias:ident) => {
        let mut $alias = {
            use std::borrow::Borrow;
            $i.borrow()
        };
    };
    ($i:ident as $alias:ident : $borrow:ty, $($tt:tt)*) => {
        borrowed!($i as $alias: $borrow);
        borrowed!($($tt)*);
    };
    ($i:ident as $alias:ident, $($tt:tt)*) => {
        borrowed!($i as $alias);
        borrowed!($($tt)*);
    };
    (mut $i:ident as $alias:ident : $borrow:ty, $($tt:tt)*) => {
        borrowed!(mut $i as $alias: $borrow);
        borrowed!($($tt)*);
    };
    (mut $i:ident as $alias:ident, $($tt:tt)*) => {
        borrowed!(mut $i as $alias);
        borrowed!($($tt)*);
    };
    ($this:ident . $i:ident as $alias:ident) => {
        let $alias = {
            use std::borrow::Borrow;
            $this.$i.borrow()
        };
    };
    ($this:ident . $i:ident as $alias:ident : $borrow:ty) => {
        let $alias: $borrow = {
            use std::borrow::Borrow;
            $this.$i.borrow()
        };
    };
    (mut $this:ident . $i:ident as $alias:ident : $borrow:ty) => {
        let mut $alias: $borrow = {
            use std::borrow::Borrow;
            $this.$i.borrow()
        };
    };
    (mut $this:ident . $i:ident as $alias:ident) => {
        let mut $alias = {
            use std::borrow::Borrow;
            $this.$i.borrow()
        };
    };
    ($this:ident . $i:ident as $alias:ident : $borrow:ty, $($tt:tt)*) => {
        borrowed!($this . $i as $alias: $borrow);
        borrowed!($($tt)*);
    };
    ($this:ident . $i:ident as $alias:ident, $($tt:tt)*) => {
        borrowed!($this . $i as $alias);
        borrowed!($($tt)*);
    };
    (mut $this:ident . $i:ident as $alias:ident : $borrow:ty, $($tt:tt)*) => {
        borrowed!(mut $this . $i as $alias: $borrow);
        borrowed!($($tt)*);
    };
    (mut $this:ident . $i:ident as $alias:ident, $($tt:tt)*) => {
        borrowed!(mut $this . $i as $alias);
        borrowed!($($tt)*);
    };

    ($i:ident : $borrow:ty) => {
        borrowed!($i as $i: $borrow)
    };
    ($i:ident) => {
        borrowed!($i as $i)
    };
    (mut $i:ident : $borrow:ty) => {
        borrowed!(mut $i as $i: $borrow)
    };
    (mut $i:ident) => {
        borrowed!(mut $i as $i)
    };
    ($i:ident : $borrow:ty, $($tt:tt)*) => {
        borrowed!($i as $i: $borrow);
        borrowed!($($tt)*);
    };
    ($i:ident, $($tt:tt)*) => {
        borrowed!($i as $i);
        borrowed!($($tt)*);
    };
    (mut $i:ident, $($tt:tt)*) => {
        borrowed!(mut $i);
        borrowed!($($tt)*);
    };

    ($this:ident . $i:ident : $borrow:ty) => {
        borrowed!($this.$i as $i: $borrow)
    };
    ($this:ident . $i:ident) => {
        borrowed!($this.$i as $i)
    };
    (mut $this:ident . $i:ident : $borrow:ty) => {
        let mut $i: $borrow = {
            use std::borrow::Borrow;
            $this.$i.borrow()
        };
    };
    (mut $this:ident . $i:ident) => {
        let mut $i = {
            use std::borrow::Borrow;
            $this.$i.borrow()
        };
    };
    ($this:ident . $i:ident : $borrow:ty, $($tt:tt)*) => {
        borrowed!($this . $i as $i: $borrow);
        borrowed!($($tt)*);
    };
    ($this:ident . $i:ident, $($tt:tt)*) => {
        borrowed!($this . $i as $i);
        borrowed!($($tt)*);
    };
    (mut $this:ident . $i:ident : $borrow:ty, $($tt:tt)*) => {
        borrowed!(mut $this . $i: $borrow);
        borrowed!($($tt)*);
    };
    (mut $this:ident . $i:ident, $($tt:tt)*) => {
        borrowed!(mut $this . $i);
        borrowed!($($tt)*);
    };

    // Handle trailing ','
    () => {};
}

#[cfg(test)]
mod tests {
    struct A {
        x: String,
    }

    impl A {
        #[allow(clippy::let_and_return)]
        fn foo(&self) -> &str {
            borrowed!(self.x);
            x
        }
    }

    #[test]
    fn test() {
        let a = A {
            x: "I am a struct".into(),
        };
        let y: String = "that can".into();
        let z: String = "talk a lot".into();
        {
            borrowed!(a.x: &str, y: &str, mut z: &str);
            let _ = a.foo();
            assert_eq!(&format!("{x} {y} {z}"), "I am a struct that can talk a lot");
            z = "";
            assert_eq!(z, "");
        }
    }

    #[test]
    #[allow(unused_variables, unused_assignments)]
    fn test_mut() {
        let a = 1;
        let b = 2;
        let c = A {
            x: "foo".to_string(),
        };

        {
            borrowed!(mut a);
            a = &1;
        }
        {
            borrowed!(mut a, b);
            a = &1;
        }
        {
            borrowed!(a, mut b);
            b = &1;
        }
        {
            borrowed!(mut c.x: &str);
            x = "bar";
        }
        {
            borrowed!(c.x: &str, mut a);
            a = &1
        }
        {
            borrowed!(a, mut c.x: &str);
            x = "bar";
        }
    }

    #[test]
    fn trailing_comma() {
        let a = 1;
        let b = 2;

        borrowed!(a, b,);

        assert_eq!((*a, *b), (1, 2))
    }

    #[test]
    fn trailing_comma_mut() {
        let a = 1;
        let b = 2;

        borrowed!(a, mut b,);
        assert_eq!(b, &2);

        b = &3;

        assert_eq!((a, b), (&1, &3))
    }

    #[test]
    #[allow(unused_variables, unused_mut)]
    fn aliases() {
        let mut a = 1;
        let mut c = A {
            x: "foo".to_string(),
        };

        {
            borrowed!(a as a2);
            borrowed!(a as a2,);
        }
        {
            borrowed!(mut a as a2);
            borrowed!(mut a as a2,);
        }
        {
            borrowed!(c.x as x2: &str);
            borrowed!(c.x as x2: &str,);
        }
        {
            borrowed!(mut c.x as x2:&str);
            borrowed!(mut c.x as x2:&str,);
        }

        {
            borrowed!(a, a as a2);
            borrowed!(a, a as a2,);
        }
        {
            borrowed!(a, mut a as a2);
            borrowed!(a, mut a as a2,);
        }
        {
            borrowed!(a, c.x as x2: &str);
            borrowed!(a, c.x as x2: &str,);
        }
        {
            borrowed!(a, mut c.x as x2: &String);
            borrowed!(a, mut c.x as x2: &String,);
        }
    }
}
