//! Per-operation response enums. Pick a variant to pick a
//! status code — IntoResponse maps each variant to its
//! documented (StatusCode, Json) pair.
#![allow(clippy::large_enum_variant)]
use axum::{http::StatusCode, response::IntoResponse, Json};
#[allow(unused_imports)]
use super::super::types::*;
/// Response for `POST /v1/ping` (operationId `ping`).
pub enum PingResponse {
    Ok(PingResponseBody),
}
impl IntoResponse for PingResponse {
    fn into_response(self) -> ::axum::response::Response {
        match self {
            Self::Ok(body) => (StatusCode::OK, Json(body)).into_response(),
        }
    }
}
