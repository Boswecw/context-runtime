//! Fail-closed error type for the context runtime.
//!
//! Every error maps to an explicit HTTP status. The runtime never returns a
//! success envelope on a rejected path — admissibility, scope, and contract
//! failures all surface as errors, mirroring PCC's fail-closed posture.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

#[derive(Debug, thiserror::Error)]
pub enum ContextError {
    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("repo root not found or not a directory: {0}")]
    RepoNotFound(String),

    #[error("target file not found in repo: {0}")]
    TargetNotFound(String),

    #[error("io error: {0}")]
    Io(String),

    /// PCC `assemble_context` rejected the gathered sources (stale / missing
    /// required / unresolved authority / disallowed override / unsupported class).
    #[error("context assembly rejected (fail-closed): {0}")]
    AssemblyRejected(String),

    /// A code-native PCC contract payload failed its own `.validate()`.
    #[error("payload contract invalid (fail-closed): {0}")]
    PayloadInvalid(String),

    #[error("context bundle not found: {0}")]
    BundleNotFound(String),

    /// The requested payload ref is not in the bundle's admitted inventory.
    /// This is the deferred forgeHQ "adapter boundary" enforced as scope policy.
    #[error("ref not admitted in bundle (scope escape rejected): {0}")]
    RefNotAdmitted(String),
}

impl ContextError {
    pub fn status(&self) -> StatusCode {
        match self {
            ContextError::BadRequest(_) => StatusCode::BAD_REQUEST,
            ContextError::RepoNotFound(_)
            | ContextError::TargetNotFound(_)
            | ContextError::BundleNotFound(_) => StatusCode::NOT_FOUND,
            ContextError::RefNotAdmitted(_) => StatusCode::CONFLICT,
            ContextError::AssemblyRejected(_) | ContextError::PayloadInvalid(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            ContextError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ContextError {
    fn into_response(self) -> Response {
        let status = self.status();
        let body = Json(serde_json::json!({
            "error": self.to_string(),
            "fail_closed": true,
        }));
        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, ContextError>;
