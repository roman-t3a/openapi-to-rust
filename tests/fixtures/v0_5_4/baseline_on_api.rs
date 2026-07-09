//! Per-tag traits. Implement one of these on your own
//! struct; the router (P5) wires it into axum.
#![allow(clippy::too_many_arguments)]
use super::errors::*;
#[allow(unused_imports)]
use super::super::types::*;
/// Request-scoped context carrying middleware-settled state from
/// [`axum::http::Extensions`].
///
/// Middleware layers (auth, tenancy, tracing, etc.) insert typed
/// values into the request extensions before the handler runs.
/// When `[server].context_extensions = true`, every generated
/// trait method receives a leading `ctx: RequestContext` built
/// by cloning those extensions out of the request parts before
/// body extraction.
#[derive(Clone)]
pub struct RequestContext(pub ::axum::http::Extensions);
impl RequestContext {
    /// Look up a value previously inserted into request extensions
    /// by middleware.
    pub fn get<T: ::std::clone::Clone + Send + Sync + 'static>(
        &self,
    ) -> ::std::option::Option<&T> {
        self.0.get::<T>()
    }
}
/// Operations under the `Health` tag.
#[async_trait::async_trait]
pub trait HealthApi: Send + Sync + 'static {
    ///
    ///
    /// `POST /v1/ping`
    async fn ping(&self, ctx: RequestContext, body: PingRequest) -> PingResponse;
}
