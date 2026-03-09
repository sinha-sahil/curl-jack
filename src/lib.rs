pub mod error;
pub mod execute;
pub mod parse;
pub mod schema;

pub use error::{CurlJackError, Result};
pub use execute::CurlResponse;
pub use parse::{parse, parse_response};
pub use schema::{
    generate_body_schema, Auth, Body, Cookie, CurlRequest, FormField, FormValue, Header,
    HttpMethod, ParsedResponse, ResponseBody, ResponseCookie, RouteParam, RouteParamKind,
    StatusClass,
};

/// Execute a `CurlRequest` using reqwest and return a `CurlResponse`.
pub async fn execute(req: &CurlRequest) -> Result<CurlResponse> {
    execute::execute(req).await
}
