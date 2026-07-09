//! Per-tag traits. Implement one of these on your own
//! struct; the router (P5) wires it into axum.
#![allow(clippy::too_many_arguments)]
use super::errors::*;
#[allow(unused_imports)]
use super::super::types::*;
/// Operations under the `Health` tag.
#[async_trait::async_trait]
pub trait HealthApi: Send + Sync + 'static {
    ///
    ///
    /// `POST /v1/ping`
    async fn ping(&self, body: PingRequest) -> PingResponse;
}
