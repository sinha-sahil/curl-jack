pub mod response;

pub use response::CurlResponse;

use std::time::Instant;

use reqwest::Client;

use crate::error::Result;
use crate::schema::{Body, CurlRequest, FormValue, Header, HttpMethod};

/// Execute a `CurlRequest` using reqwest and return a `CurlResponse`.
pub async fn execute(req: &CurlRequest) -> Result<CurlResponse> {
    let mut builder = Client::builder();

    if req.insecure {
        builder = builder.danger_accept_invalid_certs(true);
    }

    if req.follow_redirects {
        let policy = match req.max_redirects {
            Some(max) => reqwest::redirect::Policy::limited(max as usize),
            None => reqwest::redirect::Policy::limited(10),
        };
        builder = builder.redirect(policy);
    } else {
        builder = builder.redirect(reqwest::redirect::Policy::none());
    }

    if let Some(timeout) = req.connect_timeout {
        builder = builder.connect_timeout(timeout);
    }

    if let Some(timeout) = req.max_time {
        builder = builder.timeout(timeout);
    }

    if let Some(ref proxy_url) = req.proxy {
        let proxy = reqwest::Proxy::all(proxy_url)?;
        builder = builder.proxy(proxy);
    }

    let client = builder.build()?;

    let method = match &req.method {
        HttpMethod::GET => reqwest::Method::GET,
        HttpMethod::POST => reqwest::Method::POST,
        HttpMethod::PUT => reqwest::Method::PUT,
        HttpMethod::PATCH => reqwest::Method::PATCH,
        HttpMethod::DELETE => reqwest::Method::DELETE,
        HttpMethod::HEAD => reqwest::Method::HEAD,
        HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
        HttpMethod::TRACE => reqwest::Method::TRACE,
        HttpMethod::Custom(s) => reqwest::Method::from_bytes(s.as_bytes())
            .unwrap_or(reqwest::Method::GET),
    };

    let mut request = client.request(method, &req.url);

    // Set query params
    if !req.query_params.is_empty() {
        let string_pairs: Vec<(String, String)> = req
            .query_params
            .iter()
            .map(|(k, v)| {
                let val = match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                (k.clone(), val)
            })
            .collect();
        request = request.query(&string_pairs);
    }

    // Set headers
    for header in &req.headers {
        request = request.header(&header.name, &header.value);
    }

    // Set user agent
    if let Some(ref ua) = req.user_agent {
        request = request.header("User-Agent", ua);
    }

    // Set referer
    if let Some(ref referer) = req.referer {
        request = request.header("Referer", referer);
    }

    // Set cookies
    if !req.cookies.is_empty() {
        let cookie_str: String = req
            .cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; ");
        request = request.header("Cookie", cookie_str);
    }

    // Set auth
    if let Some(ref auth) = req.auth {
        match auth {
            crate::schema::Auth::Basic { username, password } => {
                request = request.basic_auth(username, Some(password));
            }
            crate::schema::Auth::Bearer(token) => {
                request = request.bearer_auth(token);
            }
        }
    }

    // Set compressed
    if req.compressed {
        request = request.header("Accept-Encoding", "gzip, deflate, br");
    }

    // Set body
    if let Some(ref body) = req.body {
        match body {
            Body::Raw(data) => {
                request = request.body(data.clone());
            }
            Body::Json(value) => {
                request = request.json(value);
            }
            Body::Binary(data) => {
                request = request.body(data.clone());
            }
            Body::FormUrlencoded(pairs) => {
                let string_pairs: Vec<(String, String)> = pairs
                    .iter()
                    .map(|(k, v)| {
                        let val = match v {
                            serde_json::Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        (k.clone(), val)
                    })
                    .collect();
                request = request.form(&string_pairs);
            }
            Body::Multipart(fields) => {
                let mut form = reqwest::multipart::Form::new();
                for field in fields {
                    match &field.value {
                        FormValue::Text(text) => {
                            form = form.text(field.name.clone(), text.clone());
                        }
                        FormValue::File { path, mime } => {
                            let file_bytes = tokio::fs::read(path).await?;
                            let filename = std::path::Path::new(path)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let mut part = reqwest::multipart::Part::bytes(file_bytes)
                                .file_name(filename);
                            if let Some(mime_type) = mime {
                                part = part.mime_str(mime_type)?;
                            }
                            form = form.part(field.name.clone(), part);
                        }
                    }
                }
                request = request.multipart(form);
            }
        }
    }

    let start = Instant::now();
    let response = request.send().await?;
    let elapsed = start.elapsed();

    let status = response.status().as_u16();
    let headers: Vec<Header> = response
        .headers()
        .iter()
        .map(|(name, value)| Header {
            name: name.to_string(),
            value: value.to_str().unwrap_or("").to_string(),
        })
        .collect();
    let body = response.bytes().await?.to_vec();

    // Write to output file if specified
    if let Some(ref output_path) = req.output {
        tokio::fs::write(output_path, &body).await?;
    }

    Ok(CurlResponse {
        status,
        headers,
        body,
        elapsed,
    })
}
