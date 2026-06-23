# wire-jack

Shape an API call with a [temple-dsl](https://github.com/sinha-sahil/temple-dsl) template, render it against runtime inputs into a typed request, and execute the result.

A temple-dsl template is the contract: it describes how to build an API call from some input. You author it once, persist its compiled form, and render it against many inputs — each render produces a typed `ApiRequest`, which the executor sends and turns into an `ApiResponse`.

```text
author template → compile → [persist] → render(input) → ApiRequest → execute → ApiResponse
```

## Installation

```toml
[dependencies]
wire-jack = "0.1.0"
```

## Usage

```rust
use wire_jack::{execute, ApiRequest, Template, Value};

// A temple-dsl template that shapes an API call from input.
let template = Template::compile(r#"
    {
      "url":     {{ concat("https://api.example.com/users/", to_string(input.id)) }},
      "method":  "GET",
      "headers": { "Authorization": {{ concat("Bearer ", input.token) }} }
    }
"#)?;

// Persist the compiled template (e.g. into a BYTEA/BLOB column)...
let blob = template.to_bytes();
let template = Template::from_bytes(&blob)?;

// ...render it against an input to produce a typed request...
let request: ApiRequest = template.render(Value::obj([
    ("id",    Value::Int(42)),
    ("token", Value::Str("abc".into())),
]))?;

// ...and execute it.
let response = execute(&request).await?;
assert!(response.ok);
```

Use `template.render_value(input)` to get the dynamic `Value` back instead of deserializing into `ApiRequest`.

## ApiRequest

The render target. Every field has a default, so a template only sets the keys it cares about.

| Field | Type | Notes |
|---|---|---|
| `url` | `String` | required |
| `method` | `String` | defaults to `"GET"` |
| `headers` | `BTreeMap<String, String>` | |
| `query` | `BTreeMap<String, String>` | |
| `cookies` | `BTreeMap<String, String>` | sent as a `Cookie` header |
| `body` | `Option<serde_json::Value>` | free-form; encoding chosen by `Content-Type` (see below) |
| `auth` | `Option<Auth>` | `{ scheme, token, username, password }` — `scheme` is `"bearer"` or `"basic"` |
| `timeout_ms` | `Option<u64>` | total request timeout |
| `follow_redirects` | `Option<bool>` | |
| `max_redirects` | `Option<u32>` | |
| `proxy` | `Option<String>` | |
| `insecure` | `Option<bool>` | skip TLS verification |
| `retry` | `Option<Retry>` | `{ max_attempts, backoff, delay_ms, max_delay_ms, retry_on_status }` |

### Body encoding

The request "type" is not a field — it is whatever `Content-Type` says. The executor encodes `body` accordingly:

| `Content-Type` | Encoding |
|---|---|
| absent (body present) | `application/json` |
| `application/json` | JSON-serialized |
| `application/x-www-form-urlencoded` | object → `k=v&…` |
| `multipart/form-data` | object; a value `{ "$file": "/path" }` becomes a file part, anything else a text part |
| binary (`application/octet-stream`, `image/*`, …) | body is a base64 string, decoded to bytes |
| `text/*`, `application/xml`, … | string sent verbatim; non-string JSON-encoded |

### Retries

`retry` drives resilience: up to `max_attempts`, retrying on a transport error or any status in `retry_on_status`, with `"fixed"` or `"exponential"` `backoff` (base `delay_ms`, capped by `max_delay_ms`).

## ApiResponse

| Field | Type | Notes |
|---|---|---|
| `status` | `u16` | |
| `status_text` | `String` | reason phrase |
| `ok` | `bool` | `true` for 2xx |
| `headers` | `BTreeMap<String, String>` | |
| `content_type` | `Option<String>` | |
| `content_length` | `Option<u64>` | |
| `cookies` | `BTreeMap<String, String>` | from `Set-Cookie` |
| `body_text` | `Option<String>` | set for non-JSON text |
| `body_json` | `Option<serde_json::Value>` | set for JSON |
| `elapsed_ms` | `u64` | |

`body_text` and `body_json` are mutually exclusive: a JSON response populates `body_json`, other text populates `body_text`, and binary/empty bodies leave both `None`.

## How it works

- **Template** — re-exported `temple_dsl::Template`; `compile`, `render`, `render_value`, `to_bytes`, `from_bytes`. This is the contract.
- **Schema** (`src/request.rs`, `src/response.rs`) — `ApiRequest` (render target) and `ApiResponse`.
- **Execute** (`src/execute.rs`) — reqwest-backed; encodes the request by `Content-Type`, applies auth/cookies/transport options and retries, and classifies the response body.

See [temple-dsl](https://github.com/sinha-sahil/temple-dsl) for the template language reference.

## License

MIT
