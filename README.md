# wire-jack

Parse curl command strings into a typed schema, serialize as JSON, and execute HTTP requests.

`wire-jack` takes a raw `curl` command — the kind you copy from browser DevTools or API docs — and turns it into a structured, serializable Rust type. From there you can inspect it, modify it, serialize it to JSON, and execute it.

## Features

- **Parse curl commands** — Handles shell quoting, escaping, backslash continuations, and `$` / `curl` prefixes
- **Typed schema** — Method, URL, headers, auth, body (JSON, form-urlencoded, multipart, raw, binary), cookies, query params, and options like proxy/timeouts
- **Route parameter detection** — Automatically identifies dynamic path segments (integer, UUID, hex, slug) and generates route templates
- **Query parameter parsing** — Extracts and type-infers query params (string, integer, boolean, nested JSON)
- **Body schema generation** — Produces a structural schema for JSON request/response bodies
- **Serde round-trip** — `CurlRequest` serializes to/from JSON, making it easy to store and hydrate templates
- **Execute requests** — Run any `CurlRequest` via reqwest and get a structured `ParsedResponse` with status, headers, cookies, body, and schema
- **Pretty display** — Human-readable `Display` output for both requests and responses

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
wire-jack = "0.1.0"
```

## Quick Start

### Parse a curl command

```rust
use wire_jack::parse;

let req = parse(r#"
    curl -X POST https://api.example.com/users \
      -H 'Content-Type: application/json' \
      -H 'Authorization: Bearer token123' \
      -d '{"name": "Alice", "email": "alice@example.com"}'
"#).unwrap();

println!("{req}");
// POST https://api.example.com/users
//
// Headers
//   Content-Type  : application/json
//   Authorization : Bearer token123
//
// Body (JSON)
//   {
//     "email": "alice@example.com",
//     "name": "Alice"
//   }
```

### Serialize to JSON and hydrate

```rust
use wire_jack::{parse, CurlRequest, Body};

let template = parse("curl -X POST -H 'Content-Type: application/json' -d '{\"name\":\"Alice\"}' https://api.example.com/users").unwrap();

// Serialize to JSON for storage
let json = serde_json::to_string_pretty(&template).unwrap();

// Later, deserialize and modify
let mut req: CurlRequest = serde_json::from_str(&json).unwrap();
req.body = Some(Body::Json(serde_json::json!({"name": "Bob"})));
```

### Execute a request

```rust
use wire_jack::{parse, execute, parse_response};

#[tokio::main]
async fn main() {
    let req = parse("curl https://jsonplaceholder.typicode.com/users/1").unwrap();
    let raw = execute(&req).await.unwrap();
    let response = parse_response(&raw);

    println!("{response}");
    // 200 OK (Success)  <- 123ms
    //
    // Body (JSON)
    //   { "id": 1, "name": "Leanne Graham", ... }
}
```

### Route parameter detection

```rust
use wire_jack::parse;

let req = parse("curl https://api.example.com/users/42/posts").unwrap();

assert_eq!(req.route_template.as_deref(), Some("https://api.example.com/users/:id/posts"));
assert_eq!(req.route_params[0].segment, "42");
```

## Supported curl flags

| Flag | Description |
|---|---|
| `-X`, `--request` | HTTP method |
| `-H`, `--header` | Request header |
| `-d`, `--data`, `--data-raw`, `--data-binary` | Request body |
| `-F`, `--form` | Multipart form field |
| `-u`, `--user` | Basic auth (`user:pass`) |
| `-b`, `--cookie` | Send cookies |
| `-L`, `--location` | Follow redirects |
| `--max-redirs` | Max redirect count |
| `-k`, `--insecure` | Skip TLS verification |
| `--compressed` | Request compressed response |
| `--connect-timeout` | Connection timeout (seconds) |
| `-m`, `--max-time` | Max request time (seconds) |
| `-x`, `--proxy` | Proxy URL |
| `-A`, `--user-agent` | User-Agent header |
| `-e`, `--referer` | Referer header |
| `-o`, `--output` | Write response to file |
| `--url` | Explicit URL |

## Interactive REPL

The included example provides an interactive parser:

```sh
cargo run --example parse_curl
# curl> curl -X POST -d '{"a":1}' https://example.com
# POST https://example.com
# ...

# JSON output mode:
cargo run --example parse_curl -- --json
```

## License

MIT
