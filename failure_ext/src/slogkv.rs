/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use super::{Compat, Error};
use futures::future::SharedError;
use std::error::Error as StdError;
use std::ops::Deref;

/// Wrapper around [Error] that implements [slog::KV] trait, so it might be used in [slog] logging
pub struct SlogKVError(pub Error);

impl slog::KV for SlogKVError {
    fn serialize(
        &self,
        _record: &slog::Record<'_>,
        serializer: &mut dyn slog::Serializer,
    ) -> slog::Result {
        let err = &self.0;

        serializer.emit_str(Error.into_str(), &format!("{}", err))?;
        #[cfg(fbcode)]
        {
            let backtrace = err.backtrace();
            if let std::backtrace::BacktraceStatus::Captured = backtrace.status() {
                serializer.emit_str(Backtrace.into_str(), &backtrace.to_string())?;
            }
        }

        let mut err = err.deref() as &dyn StdError;
        while let Some(cause) = cause_workaround(err) {
            serializer.emit_str(Cause.into_str(), &format!("{}", cause))?;
            err = cause;
        }
        serializer.emit_str(RootCause.into_str(), &format!("{:#?}", err))?;

        Ok(())
    }
}

/// Enum used in [slog::Serializer] implementation when [SlogKVError] is used
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SlogKVErrorKey {
    /// The error that is being logged
    Error,
    /// Root cause of the chain of errors
    RootCause,
    /// Backtrace taken when error occured
    Backtrace,
    /// One of causes in a chain of errors
    Cause,
}
use crate::SlogKVErrorKey::*;

impl SlogKVErrorKey {
    /// Return string representations of enum values
    pub fn into_str(self) -> &'static str {
        match self {
            Error => "error",
            RootCause => "root_cause",
            Backtrace => "backtrace",
            Cause => "cause",
        }
    }
}

impl ::std::str::FromStr for SlogKVErrorKey {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "error" => Ok(Error),
            "root_cause" => Ok(RootCause),
            "backtrace" => Ok(Backtrace),
            "cause" => Ok(Cause),
            _ => Err(()),
        }
    }
}

/// Like Fail::cause, but handles SharedError whose Fail implementation
/// does not return the right underlying error.
pub fn cause_workaround(fail: &dyn StdError) -> Option<&dyn StdError> {
    let mut cause = fail.source()?;
    if let Some(shared) = cause.downcast_ref::<SharedError<Compat<Error>>>() {
        cause = shared.0.deref();
    }
    Some(cause)
}
