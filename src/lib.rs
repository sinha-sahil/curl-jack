//! # wire-jack
//!
//! Shape an API call with a [temple-dsl](https://github.com/sinha-sahil/temple-dsl)
//! template, render it against runtime inputs into a typed [`ApiRequest`], and
//! execute the resulting request.
//!
//! A template is the contract: compile it once with [`Template::compile`],
//! persist its compiled form ([`Template::to_bytes`] / [`Template::from_bytes`]),
//! and render it against many inputs:
//!
//! ```text
//! author template -> compile -> [persist] -> render(input) -> ApiRequest -> execute
//! ```

pub mod error;
pub mod execute;
pub mod request;
pub mod response;

pub use error::{Result, WireJackError};
pub use request::{ApiRequest, Auth, Retry};
pub use response::ApiResponse;

/// The template (the contract) and dynamic value type, re-exported from temple-dsl.
pub use temple_dsl::{Template, Value};

pub async fn execute(req: &ApiRequest) -> Result<ApiResponse> {
    execute::execute(req).await
}
