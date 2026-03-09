use std::time::Duration;

use crate::error::{CurlJackError, Result};
use crate::schema::{
    generate_body_schema, Auth, Body, Cookie, CurlRequest, FormField, FormValue, Header,
    HttpMethod,
};

use super::body::build_body;
use super::url::{detect_route_params, extract_query_params};

pub fn parse_args(tokens: Vec<String>) -> Result<CurlRequest> {
    let mut req = CurlRequest::default();
    let mut explicit_method = false;
    let mut has_data = false;
    let mut data_parts: Vec<String> = Vec::new();
    let mut form_fields: Vec<FormField> = Vec::new();
    let mut data_urlencode_parts: Vec<String> = Vec::new();

    let mut iter = tokens.into_iter();

    // Skip the leading "curl" token if present
    if let Some(first) = iter.next() {
        if first != "curl" && !first.starts_with('-') {
            req.url = first;
        }
    }

    while let Some(token) = iter.next() {
        match token.as_str() {
            "-X" | "--request" => {
                let method = next_arg(&mut iter, &token)?;
                req.method = match method.parse() {
                    Ok(m) => m,
                    Err(infallible) => match infallible {},
                };
                explicit_method = true;
            }
            "--url" => {
                req.url = next_arg(&mut iter, &token)?;
            }
            "-H" | "--header" => {
                let header_str = next_arg(&mut iter, &token)?;
                let header = parse_header(&header_str)?;
                req.headers.push(header);
            }
            "-d" | "--data" | "--data-raw" => {
                let data = next_arg(&mut iter, &token)?;
                data_parts.push(data);
                has_data = true;
            }
            "--data-binary" => {
                let data = next_arg(&mut iter, &token)?;
                if let Some(path) = data.strip_prefix('@') {
                    req.body = Some(Body::Binary(path.as_bytes().to_vec()));
                } else {
                    req.body = Some(Body::Binary(data.into_bytes()));
                }
                has_data = true;
            }
            "--data-urlencode" => {
                let data = next_arg(&mut iter, &token)?;
                data_urlencode_parts.push(data);
                has_data = true;
            }
            "-F" | "--form" => {
                let field_str = next_arg(&mut iter, &token)?;
                let field = parse_form_field(&field_str)?;
                form_fields.push(field);
                has_data = true;
            }
            "-u" | "--user" => {
                let credentials = next_arg(&mut iter, &token)?;
                req.auth = Some(parse_basic_auth(&credentials));
            }
            "-b" | "--cookie" => {
                let cookie_str = next_arg(&mut iter, &token)?;
                req.cookies.extend(parse_cookies(&cookie_str));
            }
            "-L" | "--location" => {
                req.follow_redirects = true;
            }
            "--max-redirs" => {
                let val = next_arg(&mut iter, &token)?;
                req.max_redirects = val.parse().ok();
            }
            "-k" | "--insecure" => {
                req.insecure = true;
            }
            "--compressed" => {
                req.compressed = true;
            }
            "--connect-timeout" => {
                let secs = next_arg(&mut iter, &token)?;
                if let Ok(s) = secs.parse::<f64>() {
                    req.connect_timeout = Some(Duration::from_secs_f64(s));
                }
            }
            "-m" | "--max-time" => {
                let secs = next_arg(&mut iter, &token)?;
                if let Ok(s) = secs.parse::<f64>() {
                    req.max_time = Some(Duration::from_secs_f64(s));
                }
            }
            "--proxy" | "-x" => {
                req.proxy = Some(next_arg(&mut iter, &token)?);
            }
            "-A" | "--user-agent" => {
                req.user_agent = Some(next_arg(&mut iter, &token)?);
            }
            "-e" | "--referer" => {
                req.referer = Some(next_arg(&mut iter, &token)?);
            }
            "-o" | "--output" => {
                req.output = Some(next_arg(&mut iter, &token)?);
            }
            // Skip flags we don't handle but that take no argument
            "-s" | "--silent" | "-S" | "--show-error" | "-v" | "--verbose" | "-#"
            | "--progress-bar" | "-f" | "--fail" | "-g" | "--globoff" | "-N"
            | "--no-buffer" => {}
            // Skip flags we don't handle that take an argument
            "--cert" | "-E" | "--cacert" | "--capath" | "--key" | "--ciphers"
            | "--resolve" | "--interface" | "-w" | "--write-out" => {
                let _ = next_arg(&mut iter, &token);
            }
            other => {
                if !other.starts_with('-') && req.url.is_empty() {
                    req.url = other.to_string();
                }
            }
        }
    }

    if req.url.is_empty() {
        return Err(CurlJackError::MissingUrl);
    }

    // Extract query params from URL
    let query = extract_query_params(&req.url);
    req.url = query.base_url;
    req.query_params = query.params;
    req.query_schema = query.schema;

    // Detect route params in URL path
    let (route_params, route_template) = detect_route_params(&req.url);
    if !route_params.is_empty() {
        req.route_params = route_params;
        req.route_template = Some(route_template);
    }

    // Build body from accumulated data
    if !form_fields.is_empty() {
        req.body = Some(Body::Multipart(form_fields));
    } else if !data_urlencode_parts.is_empty() || (!data_parts.is_empty() && req.body.is_none()) {
        req.body = Some(build_body(data_parts, data_urlencode_parts, &req.headers));
    }

    // Auto-detect POST when data is present and no explicit method
    if has_data && !explicit_method {
        req.method = HttpMethod::POST;
    }

    // Auto-add Content-Type for -d if not present (Raw or FormUrlencoded)
    if matches!(req.body, Some(Body::Raw(_) | Body::FormUrlencoded(_))) {
        let has_content_type = req
            .headers
            .iter()
            .any(|h| h.name.eq_ignore_ascii_case("content-type"));
        if !has_content_type {
            req.headers.push(Header {
                name: "Content-Type".to_string(),
                value: "application/x-www-form-urlencoded".to_string(),
            });
        }
    }

    // Generate body schema
    if let Some(ref body) = req.body {
        req.body_schema = Some(generate_body_schema(body));
    }

    Ok(req)
}

fn next_arg(iter: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    iter.next()
        .ok_or_else(|| CurlJackError::Parse(format!("flag '{flag}' requires a value")))
}

fn parse_header(s: &str) -> Result<Header> {
    let (name, value) = s
        .split_once(':')
        .ok_or_else(|| CurlJackError::InvalidHeader(s.to_string()))?;
    Ok(Header {
        name: name.trim().to_string(),
        value: value.trim().to_string(),
    })
}

fn parse_basic_auth(credentials: &str) -> Auth {
    if let Some((user, pass)) = credentials.split_once(':') {
        Auth::Basic {
            username: user.to_string(),
            password: pass.to_string(),
        }
    } else {
        Auth::Basic {
            username: credentials.to_string(),
            password: String::new(),
        }
    }
}

fn parse_cookies(s: &str) -> Vec<Cookie> {
    s.split(';')
        .filter_map(|part| {
            let part = part.trim();
            let (name, value) = part.split_once('=')?;
            Some(Cookie {
                name: name.trim().to_string(),
                value: value.trim().to_string(),
            })
        })
        .collect()
}

fn parse_form_field(s: &str) -> Result<FormField> {
    let (name, value) = s
        .split_once('=')
        .ok_or_else(|| CurlJackError::Parse(format!("invalid form field: {s}")))?;

    let form_value = if let Some(rest) = value.strip_prefix('@') {
        if let Some((path, type_part)) = rest.split_once(";type=") {
            FormValue::File {
                path: path.to_string(),
                mime: Some(type_part.to_string()),
            }
        } else {
            FormValue::File {
                path: rest.to_string(),
                mime: None,
            }
        }
    } else {
        FormValue::Text(value.to_string())
    };

    Ok(FormField {
        name: name.to_string(),
        value: form_value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_header() {
        let h = parse_header("Content-Type: application/json").unwrap();
        assert_eq!(h.name, "Content-Type");
        assert_eq!(h.value, "application/json");
    }

    #[test]
    fn test_parse_basic_auth() {
        let auth = parse_basic_auth("user:pass");
        assert_eq!(
            auth,
            Auth::Basic {
                username: "user".to_string(),
                password: "pass".to_string()
            }
        );
    }

    #[test]
    fn test_parse_cookies() {
        let cookies = parse_cookies("name=value; session=abc123");
        assert_eq!(cookies.len(), 2);
        assert_eq!(cookies[0].name, "name");
        assert_eq!(cookies[0].value, "value");
        assert_eq!(cookies[1].name, "session");
        assert_eq!(cookies[1].value, "abc123");
    }

    #[test]
    fn test_parse_form_field_text() {
        let field = parse_form_field("key=value").unwrap();
        assert_eq!(field.name, "key");
        assert_eq!(field.value, FormValue::Text("value".to_string()));
    }

    #[test]
    fn test_parse_form_field_file() {
        let field = parse_form_field("file=@/path/to/file").unwrap();
        assert_eq!(field.name, "file");
        assert_eq!(
            field.value,
            FormValue::File {
                path: "/path/to/file".to_string(),
                mime: None
            }
        );
    }

    #[test]
    fn test_parse_form_field_file_with_mime() {
        let field = parse_form_field("file=@photo.jpg;type=image/jpeg").unwrap();
        assert_eq!(field.name, "file");
        assert_eq!(
            field.value,
            FormValue::File {
                path: "photo.jpg".to_string(),
                mime: Some("image/jpeg".to_string())
            }
        );
    }
}
