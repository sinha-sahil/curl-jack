mod args;
pub(crate) mod body;
mod response;
mod tokenizer;
pub(crate) mod url;

use crate::error::Result;
use crate::execute::CurlResponse;
use crate::schema::{CurlRequest, ParsedResponse};

/// Parse a curl command string into a structured `CurlRequest`.
///
/// Handles shell quoting, escaping, backslash line continuations,
/// and leading `$` or `curl` prefixes.
pub fn parse(input: &str) -> Result<CurlRequest> {
    let tokens = tokenizer::tokenize(input)?;
    args::parse_args(tokens)
}

/// Parse a raw `CurlResponse` into a structured `ParsedResponse`.
///
/// Classifies the response body (JSON, HTML, XML, text, binary),
/// extracts Set-Cookie headers, generates body schema for JSON responses,
/// and resolves the HTTP status reason phrase.
pub fn parse_response(raw: &CurlResponse) -> ParsedResponse {
    response::parse_response(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::*;

    #[test]
    fn test_simple_get() {
        let req = parse("curl https://example.com").unwrap();
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.method, HttpMethod::GET);
        assert!(req.body.is_none());
    }

    #[test]
    fn test_post_with_json() {
        let req = parse(
            r#"curl -X POST -H 'Content-Type: application/json' -d '{"a":1}' https://example.com"#,
        )
        .unwrap();
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.body, Some(Body::Json(serde_json::json!({"a": 1}))));
        // Content-Type was explicitly set, should not be duplicated
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].name, "Content-Type");
        assert_eq!(req.headers[0].value, "application/json");
    }

    #[test]
    fn test_auto_post_with_data() {
        let req = parse("curl -d 'foo=bar' https://example.com").unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(
            req.body,
            Some(Body::FormUrlencoded(vec![("foo".to_string(), serde_json::Value::String("bar".to_string()))]))
        );
        // Should auto-add Content-Type
        assert!(req
            .headers
            .iter()
            .any(|h| h.name == "Content-Type"
                && h.value == "application/x-www-form-urlencoded"));
    }

    #[test]
    fn test_basic_auth() {
        let req = parse("curl -u user:pass https://example.com").unwrap();
        assert_eq!(
            req.auth,
            Some(Auth::Basic {
                username: "user".to_string(),
                password: "pass".to_string()
            })
        );
    }

    #[test]
    fn test_bearer_via_header() {
        let req = parse("curl -H 'Authorization: Bearer token123' https://example.com").unwrap();
        assert_eq!(req.headers[0].name, "Authorization");
        assert_eq!(req.headers[0].value, "Bearer token123");
    }

    #[test]
    fn test_multipart_form() {
        let req = parse("curl -F 'file=@/path/to/file' -F 'name=test' https://example.com").unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        match &req.body {
            Some(Body::Multipart(fields)) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "file");
                assert_eq!(
                    fields[0].value,
                    FormValue::File {
                        path: "/path/to/file".to_string(),
                        mime: None
                    }
                );
                assert_eq!(fields[1].name, "name");
                assert_eq!(fields[1].value, FormValue::Text("test".to_string()));
            }
            _ => panic!("expected multipart body"),
        }
    }

    #[test]
    fn test_cookies() {
        let req = parse("curl -b 'session=abc; user=joe' https://example.com").unwrap();
        assert_eq!(req.cookies.len(), 2);
        assert_eq!(req.cookies[0].name, "session");
        assert_eq!(req.cookies[0].value, "abc");
    }

    #[test]
    fn test_follow_redirects() {
        let req = parse("curl -L https://example.com").unwrap();
        assert!(req.follow_redirects);
    }

    #[test]
    fn test_insecure() {
        let req = parse("curl -k https://example.com").unwrap();
        assert!(req.insecure);
    }

    #[test]
    fn test_compressed() {
        let req = parse("curl --compressed https://example.com").unwrap();
        assert!(req.compressed);
    }

    #[test]
    fn test_timeouts() {
        let req = parse("curl --connect-timeout 5 -m 30 https://example.com").unwrap();
        assert_eq!(req.connect_timeout, Some(std::time::Duration::from_secs(5)));
        assert_eq!(req.max_time, Some(std::time::Duration::from_secs(30)));
    }

    #[test]
    fn test_proxy() {
        let req = parse("curl -x http://proxy:8080 https://example.com").unwrap();
        assert_eq!(req.proxy, Some("http://proxy:8080".to_string()));
    }

    #[test]
    fn test_user_agent() {
        let req = parse("curl -A 'MyAgent/1.0' https://example.com").unwrap();
        assert_eq!(req.user_agent, Some("MyAgent/1.0".to_string()));
    }

    #[test]
    fn test_referer() {
        let req = parse("curl -e 'https://referrer.com' https://example.com").unwrap();
        assert_eq!(req.referer, Some("https://referrer.com".to_string()));
    }

    #[test]
    fn test_output() {
        let req = parse("curl -o output.json https://example.com").unwrap();
        assert_eq!(req.output, Some("output.json".to_string()));
    }

    #[test]
    fn test_url_flag() {
        let req = parse("curl --url https://example.com").unwrap();
        assert_eq!(req.url, "https://example.com");
    }

    #[test]
    fn test_line_continuation() {
        let req = parse("curl \\\n  -X POST \\\n  https://example.com").unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.url, "https://example.com");
    }

    #[test]
    fn test_dollar_prefix() {
        let req = parse("$ curl https://example.com").unwrap();
        assert_eq!(req.url, "https://example.com");
    }

    #[test]
    fn test_missing_url() {
        let result = parse("curl -X POST");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_headers() {
        let req = parse(
            "curl -H 'Accept: application/json' -H 'X-Custom: value' https://example.com",
        )
        .unwrap();
        assert_eq!(req.headers.len(), 2);
    }

    #[test]
    fn test_round_trip_json() {
        let req = parse(
            r#"curl -X POST -H 'Content-Type: application/json' -d '{"key":"val"}' -u user:pass -b 'sid=123' -L -k --compressed https://api.example.com/endpoint"#,
        )
        .unwrap();

        let json = serde_json::to_string(&req).unwrap();
        let deserialized: CurlRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.url, req.url);
        assert_eq!(deserialized.method, req.method);
        assert_eq!(deserialized.headers.len(), req.headers.len());
        assert_eq!(deserialized.body, req.body);
        assert_eq!(deserialized.auth, req.auth);
        assert_eq!(deserialized.cookies.len(), req.cookies.len());
        assert_eq!(deserialized.follow_redirects, req.follow_redirects);
        assert_eq!(deserialized.insecure, req.insecure);
        assert_eq!(deserialized.compressed, req.compressed);
    }

    #[test]
    fn test_complex_real_world_curl() {
        let input = r#"$ curl 'https://api.example.com/v1/users' \
  -H 'accept: application/json' \
  -H 'authorization: Bearer eyJhbGciOiJIUzI1NiJ9' \
  -H 'content-type: application/json' \
  --data-raw '{"name":"John","email":"john@example.com"}' \
  --compressed"#;
        let req = parse(input).unwrap();
        assert_eq!(req.url, "https://api.example.com/v1/users");
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.headers.len(), 3);
        assert!(req.compressed);
        assert_eq!(
            req.body,
            Some(Body::Json(serde_json::json!({"name": "John", "email": "john@example.com"})))
        );
    }

    #[test]
    fn test_query_params_simple() {
        let req = parse("curl 'https://api.example.com/search?q=rust&limit=10'").unwrap();
        assert_eq!(req.url, "https://api.example.com/search");
        assert_eq!(req.query_params.len(), 2);
        assert_eq!(req.query_params[0], ("q".to_string(), serde_json::json!("rust")));
        assert_eq!(req.query_params[1], ("limit".to_string(), serde_json::json!(10)));
    }

    #[test]
    fn test_query_params_schema() {
        let req = parse("curl 'https://api.example.com/users?page=1&active=true&name=John'").unwrap();
        let schema = req.query_schema.unwrap();
        assert_eq!(schema["page"], serde_json::json!("integer"));
        assert_eq!(schema["active"], serde_json::json!("boolean"));
        assert_eq!(schema["name"], serde_json::json!("string"));
    }

    #[test]
    fn test_query_params_url_decoded() {
        let req = parse("curl 'https://api.example.com/search?q=hello%20world&tag=a%26b'").unwrap();
        assert_eq!(req.query_params[0], ("q".to_string(), serde_json::json!("hello world")));
        assert_eq!(req.query_params[1], ("tag".to_string(), serde_json::json!("a&b")));
    }

    #[test]
    fn test_no_query_params() {
        let req = parse("curl https://example.com").unwrap();
        assert!(req.query_params.is_empty());
        assert!(req.query_schema.is_none());
    }

    #[test]
    fn test_query_params_with_json_value() {
        let req = parse(r#"curl 'https://api.example.com/filter?where={"status":"active"}&limit=5'"#).unwrap();
        assert_eq!(req.url, "https://api.example.com/filter");
        assert_eq!(req.query_params[0].0, "where");
        assert_eq!(req.query_params[0].1, serde_json::json!({"status": "active"}));
        assert_eq!(req.query_params[1], ("limit".to_string(), serde_json::json!(5)));
        let schema = req.query_schema.unwrap();
        assert_eq!(schema["where"]["status"], serde_json::json!("string"));
        assert_eq!(schema["limit"], serde_json::json!("integer"));
    }

    #[test]
    fn test_route_params_numeric_id() {
        let req = parse("curl https://api.example.com/users/123/posts").unwrap();
        assert_eq!(req.route_params.len(), 1);
        assert_eq!(req.route_params[0].segment, "123");
        assert_eq!(req.route_params[0].kind, crate::schema::RouteParamKind::Integer);
        assert_eq!(
            req.route_template,
            Some("https://api.example.com/users/:id/posts".to_string())
        );
    }

    #[test]
    fn test_route_params_uuid() {
        let req = parse("curl https://api.example.com/orders/550e8400-e29b-41d4-a716-446655440000/items").unwrap();
        assert_eq!(req.route_params.len(), 1);
        assert_eq!(req.route_params[0].kind, crate::schema::RouteParamKind::Uuid);
        assert_eq!(
            req.route_template,
            Some("https://api.example.com/orders/:uuid/items".to_string())
        );
    }

    #[test]
    fn test_route_params_multiple() {
        let req = parse("curl https://api.example.com/users/42/posts/99/comments").unwrap();
        assert_eq!(req.route_params.len(), 2);
        assert_eq!(req.route_params[0].segment, "42");
        assert_eq!(req.route_params[1].segment, "99");
        assert_eq!(
            req.route_template,
            Some("https://api.example.com/users/:id/posts/:id/comments".to_string())
        );
    }

    #[test]
    fn test_route_params_no_dynamic_segments() {
        let req = parse("curl https://api.example.com/users/list").unwrap();
        assert!(req.route_params.is_empty());
        assert!(req.route_template.is_none());
    }

    #[test]
    fn test_route_params_hex_hash() {
        let req = parse("curl https://api.example.com/commits/a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6").unwrap();
        assert_eq!(req.route_params.len(), 1);
        assert_eq!(req.route_params[0].kind, crate::schema::RouteParamKind::Uuid);
        assert_eq!(
            req.route_template,
            Some("https://api.example.com/commits/:uuid".to_string())
        );
    }

    #[test]
    fn test_route_params_slug() {
        let req = parse("curl https://api.example.com/products/sku-12345/details").unwrap();
        assert_eq!(req.route_params.len(), 1);
        assert_eq!(req.route_params[0].segment, "sku-12345");
        assert_eq!(req.route_params[0].kind, crate::schema::RouteParamKind::Slug);
        assert_eq!(
            req.route_template,
            Some("https://api.example.com/products/:slug/details".to_string())
        );
    }

    #[test]
    fn test_route_params_version_not_detected() {
        let req = parse("curl https://api.example.com/v1/users/123").unwrap();
        assert_eq!(req.route_params.len(), 1);
        assert_eq!(req.route_params[0].segment, "123");
        // v1 should NOT be detected as a route param
        assert!(req.route_params.iter().all(|p| p.segment != "v1"));
    }

    #[test]
    fn test_route_params_with_query() {
        let req = parse("curl 'https://api.example.com/users/42?include=posts'").unwrap();
        assert_eq!(req.route_params.len(), 1);
        assert_eq!(req.route_params[0].segment, "42");
        assert_eq!(req.query_params.len(), 1);
        assert_eq!(req.query_params[0].0, "include");
    }
}
