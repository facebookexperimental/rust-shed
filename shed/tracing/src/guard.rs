/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use std::mem;
use std::sync::{Arc, Weak};
use std::time::Instant;

use chrome_trace::{Args, Event, Phase};

use crate::context::TraceContextInner;

/// An RAII guard that logs a synchronous trace event when it goes out of scope.
/// Useful for scoped synchronous tracing of functions and blocks.
#[derive(Debug)]
pub struct TraceGuard {
    context: Weak<TraceContextInner>,
    name: String,
    args: Args,
    start: Instant,
}

impl TraceGuard {
    pub(crate) fn new(context: &Arc<TraceContextInner>, name: String, args: Args) -> Self {
        TraceGuard {
            context: Arc::downgrade(context),
            name,
            args,
            start: Instant::now(),
        }
    }

    /// Get a reference to the trace arguments associated with this guard. Allows the code
    /// being traced to manipulate the arguments that will be logged once this guard goes
    /// out of scope.
    pub fn args(&mut self) -> &mut Args {
        &mut self.args
    }
}

impl Drop for TraceGuard {
    fn drop(&mut self) {
        // Has the context that produced this guard already been dropped?
        let context = match self.context.upgrade() {
            Some(ctx) => ctx,
            None => return,
        };

        let dur = self.start.elapsed();
        let name = mem::replace(&mut self.name, Default::default());
        let args = mem::replace(&mut self.args, Default::default());
        let event = Event::new(name, Phase::Complete)
            .args(args)
            .ts(self.start.duration_since(context.epoch))
            .dur(dur);
        context.add_event(event);
    }
}
