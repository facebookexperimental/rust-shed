/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Thrift adapters that convert between thrift types and Rust types.
//!
//! Thrift adapters allow Rust code interacting with thrift types to be
//! presented as another type instead. This allows for Rust to interact with
//! thrift at a much higher semantic level with stronger type safety.
//!
//! As a result, when using an adapter, all Rust clients and services will have
//! additional parsing performed automatically when receiving a message. All
//! Rust clients and services will also be required to provide an adapted type
//! instead of the original thrift type when sending a message, preventing type
//! confusion and ensuring that what is sent is semantically correct.
//!
//! # Examples
//!
//! Using thrift adapters involve making changes to the `thrift_library` target
//! and the thrift file itself.
//!
//! The following `thrift_library` target is a complete example. Please
//! especially note the comments.
//!
//! ```skylark
//! thrift_library(
//!     name = "api",
//!     languages = ["rust"],
//!     rust_deps = [
//!         // The target in which exports your adapter.
//!         "//common/rust/shed/fbthrift_ext:adapters",
//!     ],
//!     // Required if using adapters, see 'Safety' section.
//!     rust_unittests = True,
//!     thrift_srcs = {
//!         "api.thrift": [],
//!     },
//!     deps = [
//!         // Needed to enable the @rust.Adapter annotation.
//!         "//thrift/annotation:rust",
//!     ],
//! )
//! ```
//!
//! Then, in your thrift file, you can begin to use your adapter:
//!
//! ```thrift
//! include "thrift/annotation/rust.thrift"
//!
//! @rust.Adapter{name = "::fbthrift_adapters::UuidAdapter<>"}
//! typedef binary uuid
//!
//! struct CreateWorkflowRequest {
//!   1: uuid id;
//! }
//! ```
//!
//! The field in Rust will now be the adapted type rather than the thrift type.
//!
//! ```
//! # // TODO: You could probably make this an actual thrift target
//! # mod api {
//! # #[derive(Default)]
//! # pub struct CreateWorkflowRequest { pub id: uuid::Uuid }
//! # }
//! #
//! use api::CreateWorkflowRequest;
//! use uuid::Uuid;
//!
//! let my_req = CreateWorkflowRequest {
//!     id: Uuid::new_v4(),
//!     ..Default::default()
//! };
//! ```
//!
//! Please note that this is not the only workflow possible. See the Adapters
//! wiki under the Rust @ Meta internal wiki for more information.
//!
//! All adapters are re-exported at the root level for easy usage in thrift.

pub mod chrono;
pub mod duration;
pub mod ipv4;
pub mod ipv6;
pub mod nonnegative;
pub mod ordered_float;
pub mod path;
pub mod redacted;
pub mod socket_addr;
pub mod unsigned_int;
pub mod uuid;

#[doc(inline)]
pub use crate::duration::*;
#[doc(inline)]
pub use crate::ipv4::Ipv4AddressAdapter;
#[doc(inline)]
pub use crate::ipv6::Ipv6AddressAdapter;
#[doc(inline)]
pub use crate::nonnegative::NonNegativeAdapter;
#[doc(inline)]
pub use crate::ordered_float::OrderedFloatAdapter;
#[doc(inline)]
pub use crate::path::Utf8PathAdapter;
#[doc(inline)]
pub use crate::redacted::Redacted;
#[doc(inline)]
pub use crate::redacted::RedactedAdapter;
#[doc(inline)]
pub use crate::socket_addr::SocketAddrAdapter;
#[doc(inline)]
pub use crate::unsigned_int::UnsignedIntAdapter;
#[doc(inline)]
pub use crate::uuid::UuidAdapter;
