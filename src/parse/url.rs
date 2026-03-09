use crate::schema::{generate_pairs_schema, RouteParam, RouteParamKind};

/// Result of extracting query params from a URL.
pub struct QueryExtraction {
    pub base_url: String,
    pub params: Vec<(String, serde_json::Value)>,
    pub schema: Option<serde_json::Value>,
}

/// Split query string from URL, decode params, and generate schema.
pub fn extract_query_params(url: &str) -> QueryExtraction {
    let Some(query_start) = url.find('?') else {
        return QueryExtraction {
            base_url: url.to_string(),
            params: Vec::new(),
            schema: None,
        };
    };

    let query_string = &url[query_start + 1..];
    let params: Vec<(String, serde_json::Value)> = query_string
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|segment| {
            let (k, v) = segment.split_once('=')?;
            let decoded_k = url_decode(k);
            let decoded_v = url_decode(v);
            Some((decoded_k, super::body::parse_value(&decoded_v)))
        })
        .collect();

    let schema = if params.is_empty() {
        None
    } else {
        Some(generate_pairs_schema(&params))
    };

    QueryExtraction {
        base_url: url[..query_start].to_string(),
        params,
        schema,
    }
}

/// Detect dynamic route parameters in the URL path.
/// Returns the detected params and a route template with placeholders.
pub fn detect_route_params(url: &str) -> (Vec<RouteParam>, String) {
    let path_start = url
        .find("://")
        .and_then(|i| url[i + 3..].find('/').map(|j| i + 3 + j))
        .unwrap_or(0);

    let (prefix, path) = url.split_at(path_start);

    let mut params = Vec::new();
    let mut template_segments = Vec::new();

    for (pos, segment) in path.split('/').enumerate() {
        if segment.is_empty() {
            template_segments.push(String::new());
            continue;
        }
        if let Some(kind) = classify_segment(segment) {
            params.push(RouteParam {
                segment: segment.to_string(),
                kind: kind.clone(),
                position: pos,
            });
            template_segments.push(kind.to_string());
        } else {
            template_segments.push(segment.to_string());
        }
    }

    let template = format!("{}{}", prefix, template_segments.join("/"));
    (params, template)
}

fn classify_segment(segment: &str) -> Option<RouteParamKind> {
    if !segment.is_empty() && segment.chars().all(|c| c.is_ascii_digit()) {
        return Some(RouteParamKind::Integer);
    }

    if segment.len() == 36 && is_uuid(segment) {
        return Some(RouteParamKind::Uuid);
    }

    if segment.len() == 32 && segment.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(RouteParamKind::Uuid);
    }

    if segment.len() >= 16 && segment.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(RouteParamKind::Hex);
    }

    if looks_like_slug_id(segment) {
        return Some(RouteParamKind::Slug);
    }

    None
}

fn is_uuid(s: &str) -> bool {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return false;
    }
    let expected_lens = [8, 4, 4, 4, 12];
    parts
        .iter()
        .zip(expected_lens.iter())
        .all(|(part, &len)| part.len() == len && part.chars().all(|c| c.is_ascii_hexdigit()))
}

fn looks_like_slug_id(s: &str) -> bool {
    if s.len() < 4 {
        return false;
    }
    let has_digit = s.chars().any(|c| c.is_ascii_digit());
    if !has_digit {
        return false;
    }
    let has_separator = s.contains('-') || s.contains('_');
    if !has_separator {
        return false;
    }
    let valid_chars = s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid_chars {
        return false;
    }
    if s.starts_with('v') && s.as_bytes().get(1).is_some_and(|b| b.is_ascii_digit()) {
        return false;
    }
    true
}

pub fn url_decode(s: &str) -> String {
    let mut result = Vec::new();
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'%' => {
                let hi = bytes.next().and_then(|c| (c as char).to_digit(16));
                let lo = bytes.next().and_then(|c| (c as char).to_digit(16));
                if let (Some(h), Some(l)) = (hi, lo) {
                    result.push((h * 16 + l) as u8);
                }
            }
            b'+' => result.push(b' '),
            _ => result.push(b),
        }
    }
    String::from_utf8(result).unwrap_or_else(|_| s.to_string())
}

pub fn urlencoded_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    result
}
