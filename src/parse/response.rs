use crate::execute::CurlResponse;
use crate::schema::response::{ParsedResponse, ResponseBody, ResponseCookie, StatusClass};
use crate::schema::schema_gen::value_schema;

/// Parse a raw `CurlResponse` into a structured `ParsedResponse`.
pub fn parse_response(raw: &CurlResponse) -> ParsedResponse {
    let status_text = reason_phrase(raw.status);
    let status_class = StatusClass::from_status(raw.status);

    let content_type = raw
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("content-type"))
        .map(|h| h.value.clone());

    let content_length = raw
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("content-length"))
        .and_then(|h| h.value.parse::<u64>().ok());

    let cookies = raw
        .headers
        .iter()
        .filter(|h| h.name.eq_ignore_ascii_case("set-cookie"))
        .filter_map(|h| parse_set_cookie(&h.value))
        .collect();

    let body = classify_response_body(&raw.body, content_type.as_deref());

    let body_schema = match &body {
        ResponseBody::Json(v) => Some(value_schema(v)),
        _ => None,
    };

    ParsedResponse {
        status: raw.status,
        status_text,
        status_class,
        headers: raw.headers.clone(),
        content_type,
        content_length,
        body,
        body_schema,
        cookies,
        elapsed: raw.elapsed,
    }
}

fn classify_response_body(bytes: &[u8], content_type: Option<&str>) -> ResponseBody {
    if bytes.is_empty() {
        return ResponseBody::Empty;
    }

    let ct = content_type.unwrap_or("");
    let mime = ct.split(';').next().unwrap_or("").trim();

    // Try JSON
    if mime == "application/json" || mime.ends_with("+json") {
        if let Ok(text) = std::str::from_utf8(bytes) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
                return ResponseBody::Json(value);
            }
            return ResponseBody::Text(text.to_string());
        }
    }

    // HTML
    if mime == "text/html" || mime == "application/xhtml+xml" {
        if let Ok(text) = std::str::from_utf8(bytes) {
            return ResponseBody::Html(text.to_string());
        }
    }

    // XML
    if mime == "text/xml" || mime == "application/xml" || mime.ends_with("+xml") {
        if let Ok(text) = std::str::from_utf8(bytes) {
            return ResponseBody::Xml(text.to_string());
        }
    }

    // Any other text type
    if mime.starts_with("text/") || mime == "application/javascript" || mime == "application/csv" {
        if let Ok(text) = std::str::from_utf8(bytes) {
            return ResponseBody::Text(text.to_string());
        }
    }

    // No content-type hint — try to sniff
    if ct.is_empty() {
        if let Ok(text) = std::str::from_utf8(bytes) {
            let trimmed = text.trim();
            // Sniff JSON
            if (trimmed.starts_with('{') && trimmed.ends_with('}'))
                || (trimmed.starts_with('[') && trimmed.ends_with(']'))
            {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    return ResponseBody::Json(value);
                }
            }
            // Sniff HTML
            if trimmed.starts_with("<!DOCTYPE") || trimmed.starts_with("<html") {
                return ResponseBody::Html(text.to_string());
            }
            // Sniff XML
            if trimmed.starts_with("<?xml") {
                return ResponseBody::Xml(text.to_string());
            }
            // Valid UTF-8 text
            return ResponseBody::Text(text.to_string());
        }
    }

    ResponseBody::Binary { size: bytes.len() }
}

fn parse_set_cookie(header_value: &str) -> Option<ResponseCookie> {
    let mut parts = header_value.split(';');

    let name_value = parts.next()?.trim();
    let (name, value) = name_value.split_once('=')?;

    let mut cookie = ResponseCookie {
        name: name.trim().to_string(),
        value: value.trim().to_string(),
        domain: None,
        path: None,
        expires: None,
        max_age: None,
        secure: false,
        http_only: false,
        same_site: None,
    };

    for attr in parts {
        let attr = attr.trim();
        let lower = attr.to_ascii_lowercase();

        if lower == "secure" {
            cookie.secure = true;
        } else if lower == "httponly" {
            cookie.http_only = true;
        } else if let Some((key, val)) = attr.split_once('=') {
            let key_lower = key.trim().to_ascii_lowercase();
            let val = val.trim().to_string();
            match key_lower.as_str() {
                "domain" => cookie.domain = Some(val),
                "path" => cookie.path = Some(val),
                "expires" => cookie.expires = Some(val),
                "max-age" => cookie.max_age = val.parse().ok(),
                "samesite" => cookie.same_site = Some(val),
                _ => {}
            }
        }
    }

    Some(cookie)
}

fn reason_phrase(status: u16) -> String {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::Header;
    use std::time::Duration;

    fn make_response(status: u16, headers: Vec<(&str, &str)>, body: &[u8]) -> CurlResponse {
        CurlResponse {
            status,
            headers: headers
                .into_iter()
                .map(|(n, v)| Header {
                    name: n.to_string(),
                    value: v.to_string(),
                })
                .collect(),
            body: body.to_vec(),
            elapsed: Duration::from_millis(42),
        }
    }

    #[test]
    fn test_json_response() {
        let raw = make_response(
            200,
            vec![("content-type", "application/json; charset=utf-8")],
            br#"{"id":1,"name":"John"}"#,
        );
        let parsed = parse_response(&raw);

        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.status_text, "OK");
        assert_eq!(parsed.status_class, StatusClass::Success);
        assert!(matches!(parsed.body, ResponseBody::Json(_)));
        if let ResponseBody::Json(v) = &parsed.body {
            assert_eq!(v["id"], serde_json::json!(1));
            assert_eq!(v["name"], serde_json::json!("John"));
        }
        let schema = parsed.body_schema.unwrap();
        assert_eq!(schema["id"], serde_json::json!("integer"));
        assert_eq!(schema["name"], serde_json::json!("string"));
    }

    #[test]
    fn test_html_response() {
        let raw = make_response(
            200,
            vec![("content-type", "text/html")],
            b"<html><body>Hello</body></html>",
        );
        let parsed = parse_response(&raw);
        assert!(matches!(parsed.body, ResponseBody::Html(_)));
        assert!(parsed.body_schema.is_none());
    }

    #[test]
    fn test_xml_response() {
        let raw = make_response(
            200,
            vec![("content-type", "application/xml")],
            b"<root><item>1</item></root>",
        );
        let parsed = parse_response(&raw);
        assert!(matches!(parsed.body, ResponseBody::Xml(_)));
    }

    #[test]
    fn test_text_response() {
        let raw = make_response(
            200,
            vec![("content-type", "text/plain")],
            b"Just some text",
        );
        let parsed = parse_response(&raw);
        assert!(matches!(parsed.body, ResponseBody::Text(_)));
    }

    #[test]
    fn test_binary_response() {
        let raw = make_response(
            200,
            vec![("content-type", "application/octet-stream")],
            &[0x00, 0xFF, 0x89, 0x50, 0x4E, 0x47],
        );
        let parsed = parse_response(&raw);
        assert!(matches!(parsed.body, ResponseBody::Binary { size: 6 }));
    }

    #[test]
    fn test_empty_body() {
        let raw = make_response(204, vec![], b"");
        let parsed = parse_response(&raw);
        assert_eq!(parsed.status_text, "No Content");
        assert!(matches!(parsed.body, ResponseBody::Empty));
    }

    #[test]
    fn test_sniff_json_no_content_type() {
        let raw = make_response(200, vec![], br#"{"key":"value"}"#);
        let parsed = parse_response(&raw);
        assert!(matches!(parsed.body, ResponseBody::Json(_)));
    }

    #[test]
    fn test_sniff_html_no_content_type() {
        let raw = make_response(200, vec![], b"<!DOCTYPE html><html><body></body></html>");
        let parsed = parse_response(&raw);
        assert!(matches!(parsed.body, ResponseBody::Html(_)));
    }

    #[test]
    fn test_sniff_xml_no_content_type() {
        let raw = make_response(200, vec![], b"<?xml version=\"1.0\"?><root/>");
        let parsed = parse_response(&raw);
        assert!(matches!(parsed.body, ResponseBody::Xml(_)));
    }

    #[test]
    fn test_status_classes() {
        assert_eq!(StatusClass::from_status(100), StatusClass::Informational);
        assert_eq!(StatusClass::from_status(200), StatusClass::Success);
        assert_eq!(StatusClass::from_status(301), StatusClass::Redirection);
        assert_eq!(StatusClass::from_status(404), StatusClass::ClientError);
        assert_eq!(StatusClass::from_status(500), StatusClass::ServerError);
        assert_eq!(StatusClass::from_status(999), StatusClass::Unknown);
    }

    #[test]
    fn test_set_cookie_parsing() {
        let raw = make_response(
            200,
            vec![
                ("content-type", "application/json"),
                (
                    "set-cookie",
                    "session=abc123; Path=/; HttpOnly; Secure; SameSite=Lax",
                ),
                ("set-cookie", "theme=dark; Max-Age=86400; Domain=.example.com"),
            ],
            br#"{"ok":true}"#,
        );
        let parsed = parse_response(&raw);

        assert_eq!(parsed.cookies.len(), 2);

        assert_eq!(parsed.cookies[0].name, "session");
        assert_eq!(parsed.cookies[0].value, "abc123");
        assert_eq!(parsed.cookies[0].path, Some("/".to_string()));
        assert!(parsed.cookies[0].http_only);
        assert!(parsed.cookies[0].secure);
        assert_eq!(parsed.cookies[0].same_site, Some("Lax".to_string()));

        assert_eq!(parsed.cookies[1].name, "theme");
        assert_eq!(parsed.cookies[1].value, "dark");
        assert_eq!(parsed.cookies[1].max_age, Some(86400));
        assert_eq!(
            parsed.cookies[1].domain,
            Some(".example.com".to_string())
        );
    }

    #[test]
    fn test_content_length() {
        let raw = make_response(
            200,
            vec![
                ("content-type", "text/plain"),
                ("content-length", "14"),
            ],
            b"Just some text",
        );
        let parsed = parse_response(&raw);
        assert_eq!(parsed.content_length, Some(14));
    }

    #[test]
    fn test_error_response_with_json() {
        let raw = make_response(
            422,
            vec![("content-type", "application/json")],
            br#"{"errors":[{"field":"email","message":"is invalid"}]}"#,
        );
        let parsed = parse_response(&raw);
        assert_eq!(parsed.status, 422);
        assert_eq!(parsed.status_text, "Unprocessable Entity");
        assert_eq!(parsed.status_class, StatusClass::ClientError);
        if let ResponseBody::Json(v) = &parsed.body {
            assert_eq!(v["errors"][0]["field"], serde_json::json!("email"));
        } else {
            panic!("expected JSON body");
        }
        let schema = parsed.body_schema.unwrap();
        assert_eq!(schema["errors"][0]["field"], serde_json::json!("string"));
    }

    #[test]
    fn test_json_plus_suffix() {
        let raw = make_response(
            200,
            vec![("content-type", "application/vnd.api+json")],
            br#"{"data":{"id":"1"}}"#,
        );
        let parsed = parse_response(&raw);
        assert!(matches!(parsed.body, ResponseBody::Json(_)));
    }

    #[test]
    fn test_display_output() {
        let raw = make_response(
            200,
            vec![
                ("content-type", "application/json; charset=utf-8"),
                ("x-request-id", "req_abc123"),
                ("set-cookie", "session=abc123; Path=/; HttpOnly; Secure"),
            ],
            br#"{"id":42,"name":"John"}"#,
        );
        let parsed = parse_response(&raw);
        let output = format!("{parsed}");

        assert!(output.contains("200 OK (Success)"));
        assert!(output.contains("x-request-id"));
        assert!(output.contains("Body (JSON)"));
        assert!(output.contains("Body Schema"));
        assert!(output.contains("id   : integer"));
        assert!(output.contains("name : string"));
        assert!(output.contains("session = abc123"));
        assert!(output.contains("HttpOnly"));
    }
}
