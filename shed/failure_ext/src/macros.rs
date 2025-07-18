/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

/// Downcast matching.
///
/// Usage:
/// ```
/// # use anyhow::Error;
/// # use std::fmt::{Debug, Display};
/// # use failure_ext::err_downcast_ref;
/// # fn foo<
/// #      Type: Display + Debug + Send + Sync + 'static,
/// #      YourType: Display + Debug + Send + Sync + 'static
/// # >(err: Error) {
/// let res = err_downcast_ref! {
///    err,
///    ty: Type => { /* use ty as &Type */ },
///    yours: YourType => { /* use yours as &YourType */ },
/// };
/// # }
/// # fn main() {}
/// ```
///
/// Where `err` is a `&anyhow::Error`.
/// When one of the type arms match, then it returns Some(value from expr), otherwise None.
/// It's like downcast_ref but for multiple types.
#[macro_export]
macro_rules! err_downcast_ref {
    // Base case - all patterns consumed
    ( $err:expr_2021 ) => {
        { let _ = $err; None }
    };
    // Eliminate trailing comma
    ( $err:expr_2021, $($v:ident : $ty:ty => $action:expr_2021),* , ) => {
        err_downcast_ref!($err, $($v : $ty => $action),*)
    };
    // Default case - match one type pattern, and recur with the rest of the list.
    // The rest of the list consumes the , separating it from the first pattern and
    // is itself comma-separated, with no trailing comma
    ( $err:expr_2021, $v:ident : $ty:ty => $action:expr_2021 $(, $rv:ident : $rty:ty => $raction:expr_2021)* ) => {{
        match $err.downcast_ref::<$ty>() {
            Some($v) => Some($action),
            None => err_downcast_ref!($err $(, $rv : $rty => $raction)*),
        }
    }};
}

/// Downcast matching.
///
/// Usage:
/// ```
/// # use anyhow::Error;
/// # use std::fmt::{Debug, Display};
/// # use failure_ext::err_downcast;
/// # fn foo<
/// #      Type: Display + Debug + Send + Sync + 'static,
/// #      YourType: Display + Debug + Send + Sync + 'static
/// # >(err: Error) {
/// let res = err_downcast! {
///    err,
///    ty: Type => { /* use ty as Type */ },
///    yours: YourType => { /* use yours as YourType */ },
/// };
/// # }
/// # fn main() {}
/// ```
///
/// Where `err` is a `anyhow::Error`.
/// When one of the type arms match, then it returns Ok(value from expr), otherwise Err(err).
/// It's like downcast but for multiple types.
#[macro_export]
macro_rules! err_downcast {
    // Base case - all patterns consumed
    ( $err:expr_2021 ) => {
        Err($err)
    };
    // Eliminate trailing comma
    ( $err:expr_2021, $($v:ident : $ty:ty => $action:expr_2021),* , ) => {
        err_downcast!($err, $($v : $ty => $action),*)
    };
    // Default case - match one type pattern, and recur with the rest of the list.
    // The rest of the list consumes the , separating it from the first pattern and
    // is itself comma-separated, with no trailing comma
    ( $err:expr_2021, $v:ident : $ty:ty => $action:expr_2021 $(, $rv:ident : $rty:ty => $raction:expr_2021)* ) => {{
        match $err.downcast::<$ty>() {
            Ok($v) => Ok($action),
            Err(other) => err_downcast!(other $(, $rv : $rty => $raction)*),
        }
    }};
}

#[allow(clippy::disallowed_names)]
#[cfg(test)]
mod test {
    use anyhow::Error;
    use thiserror::Error;

    #[derive(Error, Debug)]
    #[error("Foo badness")]
    struct Foo;
    #[derive(Error, Debug)]
    #[error("Bar badness")]
    struct Bar;
    #[derive(Error, Debug)]
    #[error("Blat badness")]
    struct Blat;
    #[derive(Error, Debug)]
    #[error("Outer badness")]
    struct Outer;

    #[test]
    fn downcast_ref_syntax() {
        let blat = Error::from(Blat);

        // Single, tailing ,
        let _ = err_downcast_ref! {
            blat,
            v: Foo => v.to_string(),
        };

        // Single, no tailing ,
        let _ = err_downcast_ref! {
            blat,
            v: Foo => v.to_string()
        };

        // Multi, tailing ,
        let _ = err_downcast_ref! {
            blat,
            v: Foo => v.to_string(),
            v: Blat => v.to_string(),
        };

        // Multi, no tailing ,
        let _ = err_downcast_ref! {
            blat,
            v: Foo => v.to_string(),
            v: Blat => v.to_string()
        };
    }

    #[test]
    fn downcast_ref_basic() {
        let blat = Error::from(Blat);

        let msg = err_downcast_ref! {
            blat,
            foo: Foo => foo.to_string(),
            bar: Bar => bar.to_string(),
            blat: Blat => blat.to_string(),
            outer: Outer => outer.to_string(),
        };

        assert_eq!(msg.unwrap(), "Blat badness".to_string());
    }

    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn downcast_ref_context() {
        let foo = Error::from(Foo);
        let outer = foo.context(Outer);

        let msg1 = err_downcast_ref! {
            outer,
            foo: Foo => foo.to_string(), // expected
            bar: Bar => bar.to_string(),
            blat: Blat => blat.to_string(),
            outer: Outer => outer.to_string(),
        };
        let msg2 = err_downcast_ref! {
            outer,
            blat: Blat => blat.to_string(),
            outer: Outer => outer.to_string(), // expected
            foo: Foo => foo.to_string(),
            bar: Bar => bar.to_string(),
        };

        assert_eq!(msg1.unwrap(), "Foo badness".to_string());
        assert_eq!(msg2.unwrap(), "Outer badness".to_string());
    }

    #[test]
    fn downcast_ref_miss() {
        let blat = Error::from(Blat);

        let msg = err_downcast_ref! {
            blat,
            v: Foo => { let _: &Foo = v; v.to_string() },
            v: Bar => { let _: &Bar = v; v.to_string() },
        };

        assert!(msg.is_none());
        assert!(blat.downcast_ref::<Blat>().is_some());
    }

    #[test]
    fn downcast_syntax() {
        // Single, tailing ,
        let blat = Error::from(Blat);
        let _ = err_downcast! {
            blat,
            v: Foo => v.to_string(),
        };

        // Single, no tailing ,
        let blat = Error::from(Blat);
        let _ = err_downcast! {
            blat,
            v: Foo => v.to_string()
        };

        // Multi, tailing ,
        let blat = Error::from(Blat);
        let _ = err_downcast! {
            blat,
            v: Foo => v.to_string(),
            v: Blat => v.to_string(),
        };

        // Multi, no tailing ,
        let blat = Error::from(Blat);
        let _ = err_downcast! {
            blat,
            v: Foo => v.to_string(),
            v: Blat => v.to_string()
        };
    }

    #[test]
    fn downcast_basic() {
        let blat = Error::from(Blat);

        let msg = err_downcast! {
            blat,
            foo: Foo => foo.to_string(),
            bar: Bar => bar.to_string(),
            blat: Blat => blat.to_string(),
            outer: Outer => outer.to_string(),
        };

        assert_eq!(msg.unwrap(), "Blat badness".to_string());
    }

    #[test]
    fn downcast_context() {
        let foo = Error::from(Foo);
        let outer = foo.context(Outer);

        let msg = err_downcast! {
            outer,
            v: Foo => { let _: Foo = v; v.to_string() },
            v: Bar => { let _: Bar = v; v.to_string() },
            v: Blat => { let _: Blat = v; v.to_string() },
            v: Outer => { let _: Outer = v; v.to_string() },
        };

        assert_eq!(msg.unwrap(), "Foo badness".to_string());
    }

    #[test]
    fn downcast_miss() {
        let blat = Error::from(Blat);

        let msg = err_downcast! {
            blat,
            foo: Foo => foo.to_string(),
            bar: Bar => bar.to_string(),
            outer: Outer => outer.to_string(),
        };

        assert!(msg.is_err());
        assert!(msg.unwrap_err().downcast::<Blat>().is_ok());
    }
}
