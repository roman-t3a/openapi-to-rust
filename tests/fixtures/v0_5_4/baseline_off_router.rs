//! Router factories — one per trait. Each takes any
//! `T: <TraitName> + Clone + Send + Sync + 'static` and
//! returns an `axum::Router` with state pre-attached.
use super::api::*;
use super::errors::*;
#[allow(unused_imports)]
use super::super::types::*;
/// Build an axum::Router for the `HealthApi` trait.
pub fn health_api_router<T>(api: T) -> ::axum::Router
where
    T: HealthApi + Clone + Send + Sync + 'static,
{
    ::axum::Router::new()
        .route("/v1/ping", ::axum::routing::post(ping_handler::<T>))
        .with_state(api)
}
async fn ping_handler<T>(
    ::axum::extract::State(api): ::axum::extract::State<T>,
    ::axum::Json(body): ::axum::Json<PingRequest>,
) -> ::axum::response::Response
where
    T: super::api::HealthApi + Clone + Send + Sync + 'static,
{
    ::axum::response::IntoResponse::into_response(api.ping(body).await)
}
