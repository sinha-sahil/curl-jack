use serde_json::{Map, Value};

use super::body::{Body, FormValue};

/// Generate a type-level schema from a `Body`.
/// Replaces every leaf value with its type name and recurses into objects/arrays.
pub fn generate_body_schema(body: &Body) -> Value {
    match body {
        Body::Raw(_) => Value::String("string".into()),
        Body::Binary(_) => Value::String("binary".into()),
        Body::Json(v) => value_schema(v),
        Body::FormUrlencoded(pairs) => {
            let mut map = Map::new();
            for (k, v) in pairs {
                map.insert(k.clone(), value_schema(v));
            }
            Value::Object(map)
        }
        Body::Multipart(fields) => {
            let mut map = Map::new();
            for f in fields {
                let schema = match &f.value {
                    FormValue::Text(_) => Value::String("string".into()),
                    FormValue::File { mime, .. } => {
                        let mut m = Map::new();
                        m.insert("type".into(), Value::String("file".into()));
                        if let Some(mime) = mime {
                            m.insert("mime".into(), Value::String(mime.clone()));
                        }
                        Value::Object(m)
                    }
                };
                map.insert(f.name.clone(), schema);
            }
            Value::Object(map)
        }
    }
}

/// Generate a type-level schema for a list of key-value pairs (e.g. query params).
pub fn generate_pairs_schema(pairs: &[(String, Value)]) -> Value {
    let mut map = Map::new();
    for (k, v) in pairs {
        map.insert(k.clone(), value_schema(v));
    }
    Value::Object(map)
}

pub fn value_schema(v: &Value) -> Value {
    match v {
        Value::Null => Value::String("null".into()),
        Value::Bool(_) => Value::String("boolean".into()),
        Value::Number(n) => {
            if n.is_f64() && !n.is_i64() && !n.is_u64() {
                Value::String("float".into())
            } else {
                Value::String("integer".into())
            }
        }
        Value::String(s) => {
            // If the string looks like it could be a specific format, annotate it
            if s.is_empty() {
                Value::String("string".into())
            } else {
                Value::String(infer_string_format(s).into())
            }
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                Value::Array(vec![])
            } else {
                // Schema of the first element as representative
                Value::Array(vec![value_schema(&arr[0])])
            }
        }
        Value::Object(map) => {
            let mut schema_map = Map::new();
            for (k, v) in map {
                schema_map.insert(k.clone(), value_schema(v));
            }
            Value::Object(schema_map)
        }
    }
}

fn infer_string_format(s: &str) -> &'static str {
    if s.contains('@') && s.contains('.') {
        return "string(email)";
    }
    if s.starts_with("http://") || s.starts_with("https://") {
        return "string(url)";
    }
    // Check for numeric strings
    if s.parse::<i64>().is_ok() {
        return "string(numeric)";
    }
    if s.parse::<f64>().is_ok() {
        return "string(numeric)";
    }
    "string"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::body::{FormField, FormValue};
    use serde_json::json;

    #[test]
    fn test_json_object_schema() {
        let body = Body::Json(json!({
            "name": "John",
            "age": 30,
            "active": true,
            "score": 9.5,
            "address": {
                "city": "Pune",
                "pin": "411021"
            },
            "tags": ["a", "b"]
        }));
        let schema = generate_body_schema(&body);
        assert_eq!(schema["name"], json!("string"));
        assert_eq!(schema["age"], json!("integer"));
        assert_eq!(schema["active"], json!("boolean"));
        assert_eq!(schema["score"], json!("float"));
        assert_eq!(schema["address"]["city"], json!("string"));
        assert_eq!(schema["address"]["pin"], json!("string(numeric)"));
        assert_eq!(schema["tags"], json!(["string"]));
    }

    #[test]
    fn test_form_urlencoded_schema() {
        let body = Body::FormUrlencoded(vec![
            ("format".into(), json!("json")),
            ("data".into(), json!({"items": [{"id": 1, "name": "x"}]})),
        ]);
        let schema = generate_body_schema(&body);
        assert_eq!(schema["format"], json!("string"));
        assert_eq!(schema["data"]["items"], json!([{"id": "integer", "name": "string"}]));
    }

    #[test]
    fn test_multipart_schema() {
        let body = Body::Multipart(vec![
            FormField { name: "name".into(), value: FormValue::Text("test".into()) },
            FormField {
                name: "file".into(),
                value: FormValue::File { path: "photo.jpg".into(), mime: Some("image/jpeg".into()) },
            },
        ]);
        let schema = generate_body_schema(&body);
        assert_eq!(schema["name"], json!("string"));
        assert_eq!(schema["file"]["type"], json!("file"));
        assert_eq!(schema["file"]["mime"], json!("image/jpeg"));
    }

    #[test]
    fn test_raw_schema() {
        let schema = generate_body_schema(&Body::Raw("hello".into()));
        assert_eq!(schema, json!("string"));
    }

    #[test]
    fn test_email_detection() {
        let body = Body::Json(json!({"email": "john@example.com"}));
        let schema = generate_body_schema(&body);
        assert_eq!(schema["email"], json!("string(email)"));
    }

    #[test]
    fn test_url_detection() {
        let body = Body::Json(json!({"website": "https://example.com"}));
        let schema = generate_body_schema(&body);
        assert_eq!(schema["website"], json!("string(url)"));
    }

    #[test]
    fn test_numeric_string_detection() {
        let body = Body::Json(json!({"pin": "411021", "waybill": ""}));
        let schema = generate_body_schema(&body);
        assert_eq!(schema["pin"], json!("string(numeric)"));
        assert_eq!(schema["waybill"], json!("string"));
    }

    #[test]
    fn test_empty_array() {
        let body = Body::Json(json!({"items": []}));
        let schema = generate_body_schema(&body);
        assert_eq!(schema["items"], json!([]));
    }

    #[test]
    fn test_null_value() {
        let body = Body::Json(json!({"deleted_at": null}));
        let schema = generate_body_schema(&body);
        assert_eq!(schema["deleted_at"], json!("null"));
    }
}
