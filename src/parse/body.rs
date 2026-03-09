use crate::schema::{Body, Header};

use super::url::urlencoded_encode;

/// Assemble body from accumulated `-d` and `--data-urlencode` parts,
/// then classify the result.
pub fn build_body(
    data_parts: Vec<String>,
    data_urlencode_parts: Vec<String>,
    headers: &[Header],
) -> Body {
    let mut all_data = data_parts.join("&");
    if !data_urlencode_parts.is_empty() {
        let encoded: Vec<String> = data_urlencode_parts
            .iter()
            .map(|p| {
                if let Some((k, v)) = p.split_once('=') {
                    format!("{}={}", urlencoded_encode(k), urlencoded_encode(v))
                } else {
                    urlencoded_encode(p)
                }
            })
            .collect();
        if !all_data.is_empty() {
            all_data.push('&');
        }
        all_data.push_str(&encoded.join("&"));
    }
    classify_body(&all_data, headers)
}

/// Classify raw `-d` data into a structured Body variant.
/// Priority: JSON (if Content-Type says json or data parses as JSON object/array)
/// -> FormUrlencoded (if all &-delimited segments have key=value)
/// -> Raw fallback.
fn classify_body(data: &str, headers: &[Header]) -> Body {
    let content_type = headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("content-type"))
        .map(|h| h.value.as_str())
        .unwrap_or("");

    let is_json_content_type = content_type.contains("application/json");

    if is_json_content_type {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(data) {
            return Body::Json(value);
        }
    }

    let trimmed = data.trim();
    if !is_json_content_type {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
            if value.is_object() || value.is_array() {
                return Body::Json(value);
            }
        }
    }

    let segments: Vec<&str> = data.split('&').collect();
    let pairs: Vec<(String, serde_json::Value)> = segments
        .iter()
        .filter_map(|s| {
            let (k, v) = s.split_once('=')?;
            Some((k.to_string(), parse_value(v)))
        })
        .collect();
    if pairs.len() == segments.len() {
        return Body::FormUrlencoded(pairs);
    }

    Body::Raw(data.to_string())
}

/// Try to parse a string value as structured data (JSON object/array/number/bool/null).
/// Falls back to a plain JSON string if it doesn't parse.
pub fn parse_value(s: &str) -> serde_json::Value {
    let trimmed = s.trim();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        return v;
    }
    serde_json::Value::String(s.to_string())
}
