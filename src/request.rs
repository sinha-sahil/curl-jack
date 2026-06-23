use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    pub url: String,
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    #[serde(default)]
    pub cookies: BTreeMap<String, String>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    #[serde(default)]
    pub auth: Option<Auth>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub follow_redirects: Option<bool>,
    #[serde(default)]
    pub max_redirects: Option<u32>,
    #[serde(default)]
    pub proxy: Option<String>,
    #[serde(default)]
    pub insecure: Option<bool>,
    #[serde(default)]
    pub retry: Option<Retry>,
}

fn default_method() -> String {
    "GET".to_string()
}

impl Default for ApiRequest {
    fn default() -> Self {
        Self {
            url: String::new(),
            method: default_method(),
            headers: BTreeMap::new(),
            query: BTreeMap::new(),
            cookies: BTreeMap::new(),
            body: None,
            auth: None,
            timeout_ms: None,
            follow_redirects: None,
            max_redirects: None,
            proxy: None,
            insecure: None,
            retry: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Retry {
    pub max_attempts: u32,
    #[serde(default)]
    pub backoff: String,
    #[serde(default)]
    pub delay_ms: Option<u64>,
    #[serde(default)]
    pub max_delay_ms: Option<u64>,
    #[serde(default)]
    pub retry_on_status: Vec<u16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Auth {
    pub scheme: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use temple_dsl::{Template, Value};

    #[test]
    fn renders_minimal_request() {
        let tmpl = Template::compile(r#"{ "url": "https://example.com" }"#).unwrap();
        let req: ApiRequest = tmpl.render(Value::Null).unwrap();
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.method, "GET");
        assert!(req.headers.is_empty());
        assert!(req.body.is_none());
    }

    #[test]
    fn renders_full_request_from_input() {
        let tmpl = Template::compile(
            r#"
            {
              "url":     {{ concat("https://api.example.com/users/", to_string(input.id)) }},
              "method":  "POST",
              "headers": { "Authorization": {{ concat("Bearer ", input.token) }} },
              "query":   { "verbose": "true" },
              "body":    { "name": {{ input.name }} },
              "auth":    { "scheme": "bearer", "token": {{ input.token }} },
              "timeout_ms": 5000,
              "follow_redirects": true,
              "retry": { "max_attempts": 3, "backoff": "exponential", "retry_on_status": [429, 503] }
            }
        "#,
        )
        .unwrap();

        let req: ApiRequest = tmpl
            .render(Value::obj([
                ("id", Value::Int(42)),
                ("token", Value::Str("abc".into())),
                ("name", Value::Str("Ada".into())),
            ]))
            .unwrap();

        assert_eq!(req.url, "https://api.example.com/users/42");
        assert_eq!(req.method, "POST");
        assert_eq!(req.headers.get("Authorization").unwrap(), "Bearer abc");
        assert_eq!(req.query.get("verbose").unwrap(), "true");
        assert_eq!(req.body.unwrap()["name"], serde_json::json!("Ada"));
        assert_eq!(req.auth.as_ref().unwrap().scheme, "bearer");
        assert_eq!(req.auth.unwrap().token.unwrap(), "abc");
        assert_eq!(req.timeout_ms, Some(5000));
        assert_eq!(req.follow_redirects, Some(true));
        let retry = req.retry.unwrap();
        assert_eq!(retry.max_attempts, 3);
        assert_eq!(retry.backoff, "exponential");
        assert_eq!(retry.retry_on_status, vec![429, 503]);
    }
}
