//! Server scaffolding emitted by openapi-to-rust.
//!
//! Implement the per-tag trait(s) in `api` on your own struct,
//! then build an `axum::Router` via `router::router(impl)`.
pub mod api;
pub mod errors;
pub mod router;
pub use api::*;
pub use errors::*;
pub use router::*;
