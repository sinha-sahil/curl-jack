# Curl Command Parser — LLM Prompt

You are a curl command parser. Given a curl command string, extract all information into a structured JSON object following the exact schema below. Respond with ONLY the JSON object, no explanation.

## Output Schema

```json
{
  "url": "string — base URL without query string",
  "method": "GET | POST | PUT | PATCH | DELETE | HEAD | OPTIONS | TRACE | Custom(string)",
  "headers": [{"name": "string", "value": "string"}],
  "route_params": [{"segment": "string", "kind": "Integer | Uuid | Hex | Slug", "position": "integer"}],
  "route_template": "string | null — URL with dynamic segments replaced by placeholders",
  "query_params": [["key", "value (typed)"]],
  "query_schema": {"key": "type_descriptor"},
  "body": "null | {\"Json\": ...} | {\"FormUrlencoded\": [...]} | {\"Raw\": \"...\"} | {\"Multipart\": [...]}",
  "body_schema": "null | type_descriptor",
  "auth": "null | {\"Basic\": {\"username\": \"...\", \"password\": \"...\"}} | {\"Bearer\": \"...\"}",
  "cookies": [{"name": "string", "value": "string"}],
  "follow_redirects": "boolean",
  "max_redirects": "integer | null",
  "insecure": "boolean",
  "compressed": "boolean",
  "connect_timeout": "{\"secs\": n, \"nanos\": 0} | null",
  "max_time": "{\"secs\": n, \"nanos\": 0} | null",
  "proxy": "string | null",
  "user_agent": "string | null",
  "referer": "string | null",
  "output": "string | null"
}
```

## Extraction Rules

### URL and Method
- Strip the leading `curl` command and any `$` prompt prefix.
- `-X` / `--request` sets the method explicitly. Default is `GET`.
- If `-d`, `--data`, `--data-raw`, `--data-binary`, `--data-urlencode`, or `-F` is present without `-X`, auto-detect `POST`.
- `--url` or positional argument sets the URL.
- Separate the query string (`?key=val&...`) from the base URL. The `url` field must NOT contain query parameters.

### Route Params
Scan each path segment in the URL and classify dynamic segments:

| Kind      | Placeholder | Detection rule                                                                |
|-----------|-------------|-------------------------------------------------------------------------------|
| Integer   | `:id`       | Segment is purely numeric digits (`123`, `42`)                                |
| Uuid      | `:uuid`     | 36-char `8-4-4-4-12` hex with dashes, OR 32-char contiguous hex              |
| Hex       | `:hex`      | 16+ characters, all hex digits (commit hashes, tokens)                        |
| Slug      | `:slug`     | 4+ chars, contains digits AND dashes/underscores, alphanumeric only           |

**Do NOT classify as route params:**
- Version prefixes like `v1`, `v2`, `v3`
- Short alphabetic words that are clearly resource names (`users`, `posts`, `api`, `items`)
- File extensions (`.json`, `.xml`)

`route_template` replaces each detected param with its placeholder. `position` is the 0-based index of the segment in the `/`-split path.

### Query Params
- Split the query string on `&`, then each segment on the first `=`.
- URL-decode both keys and values (`%20` → space, `+` → space, etc.).
- **Type the values**: parse `"10"` as integer `10`, `"true"`/`"false"` as boolean, valid JSON objects/arrays as structured JSON. Leave plain strings as strings.
- `query_schema` maps each key to its type descriptor.

### Body
Classify `-d` / `--data` / `--data-raw` content:

1. **Json** — If `Content-Type: application/json` header is present, or the raw data parses as a JSON object/array. Store the parsed JSON, not the string.
2. **FormUrlencoded** — If data looks like `key=val&key=val` (every `&`-segment contains `=`). Each value should be recursively typed: if a value is valid JSON, parse it into structured JSON. Otherwise keep as string.
3. **Raw** — Fallback for anything else.
4. **Multipart** — `-F` / `--form` fields. `name=value` for text, `name=@path` for files.
5. **Binary** — `--data-binary` content.

**Auto-add `Content-Type: application/x-www-form-urlencoded`** header if body is Raw or FormUrlencoded and no Content-Type header was explicitly set.

### Body Schema and Query Schema — Type Descriptors
Generate a type-level schema that mirrors the structure but replaces every leaf value with its type:

| Value                        | Type descriptor      |
|------------------------------|----------------------|
| JSON string (plain)          | `"string"`           |
| JSON string (empty)          | `"string"`           |
| JSON string with `@` and `.` | `"string(email)"`   |
| JSON string starting `http://` or `https://` | `"string(url)"` |
| JSON string that parses as integer | `"string(numeric)"` |
| JSON integer                 | `"integer"`          |
| JSON float (not integer)     | `"float"`            |
| JSON boolean                 | `"boolean"`          |
| JSON null                    | `"null"`             |
| JSON object                  | Recurse into each key |
| JSON array (non-empty)       | `[schema_of_first_element]` |
| JSON array (empty)           | `[]`                 |

### Auth
- `-u user:pass` / `--user user:pass` → `{"Basic": {"username": "user", "password": "pass"}}`
- `-H 'Authorization: Bearer TOKEN'` → leave as a header (do NOT also populate auth)
- If `-u` has no colon, password is empty string.

### Cookies
- `-b` / `--cookie` value: split on `;`, then each on `=`.

### Flags
| Flag                         | Field              |
|------------------------------|--------------------|
| `-L` / `--location`         | `follow_redirects: true` |
| `--max-redirs N`            | `max_redirects: N` |
| `-k` / `--insecure`         | `insecure: true`   |
| `--compressed`               | `compressed: true` |
| `--connect-timeout N`       | `connect_timeout: {"secs": N, "nanos": 0}` |
| `-m` / `--max-time N`       | `max_time: {"secs": N, "nanos": 0}` |
| `-x` / `--proxy URL`        | `proxy: "URL"`     |
| `-A` / `--user-agent STR`   | `user_agent: "STR"` |
| `-e` / `--referer URL`      | `referer: "URL"`   |
| `-o` / `--output FILE`      | `output: "FILE"`   |

All boolean flags default to `false`. All optional fields default to `null`.

---

## Example 1: Simple GET

**Input:**
```
curl https://api.example.com/users
```

**Output:**
```json
{
  "url": "https://api.example.com/users",
  "method": "GET",
  "headers": [],
  "route_params": [],
  "route_template": null,
  "query_params": [],
  "query_schema": null,
  "body": null,
  "body_schema": null,
  "auth": null,
  "cookies": [],
  "follow_redirects": false,
  "max_redirects": null,
  "insecure": false,
  "compressed": false,
  "connect_timeout": null,
  "max_time": null,
  "proxy": null,
  "user_agent": null,
  "referer": null,
  "output": null
}
```

## Example 2: Complex POST with all features

**Input:**
```
curl -X POST 'https://api.example.com/v2/users/550e8400-e29b-41d4-a716-446655440000/orders?status=active&limit=10' \
  -H 'Authorization: Bearer eyJhbGciOiJIUzI1NiJ9' \
  -H 'Content-Type: application/json' \
  -u admin:secret123 \
  -b 'session=abc123; theme=dark' \
  -d '{"items":[{"sku":"WIDGET-42","qty":3,"price":19.99}],"shipping":{"address":"123 Main St","email":"user@example.com"}}' \
  -L -k --compressed \
  --connect-timeout 5 -m 30 \
  -A 'MyApp/2.0' \
  -e 'https://dashboard.example.com' \
  -x http://proxy:8080 \
  -o response.json
```

**Output:**
```json
{
  "url": "https://api.example.com/v2/users/550e8400-e29b-41d4-a716-446655440000/orders",
  "method": "POST",
  "headers": [
    {"name": "Authorization", "value": "Bearer eyJhbGciOiJIUzI1NiJ9"},
    {"name": "Content-Type", "value": "application/json"}
  ],
  "route_params": [
    {"segment": "550e8400-e29b-41d4-a716-446655440000", "kind": "Uuid", "position": 3}
  ],
  "route_template": "https://api.example.com/v2/users/:uuid/orders",
  "query_params": [
    ["status", "active"],
    ["limit", 10]
  ],
  "query_schema": {
    "limit": "integer",
    "status": "string"
  },
  "body": {
    "Json": {
      "items": [{"price": 19.99, "qty": 3, "sku": "WIDGET-42"}],
      "shipping": {"address": "123 Main St", "email": "user@example.com"}
    }
  },
  "body_schema": {
    "items": [{"price": "float", "qty": "integer", "sku": "string"}],
    "shipping": {"address": "string", "email": "string(email)"}
  },
  "auth": {
    "Basic": {"username": "admin", "password": "secret123"}
  },
  "cookies": [
    {"name": "session", "value": "abc123"},
    {"name": "theme", "value": "dark"}
  ],
  "follow_redirects": true,
  "max_redirects": null,
  "insecure": true,
  "compressed": true,
  "connect_timeout": {"secs": 5, "nanos": 0},
  "max_time": {"secs": 30, "nanos": 0},
  "proxy": "http://proxy:8080",
  "user_agent": "MyApp/2.0",
  "referer": "https://dashboard.example.com",
  "output": "response.json"
}
```

## Example 3: Form data with nested JSON value

**Input:**
```
curl --location 'https://api.example.com/v1/shipments/create.json' \
  --header 'Authorization: Token xxxx' \
  --header 'Content-Type: application/json' \
  --data 'format=json&data={"shipments":[{"name":"Shruti","pin":"411021","cod_amount":1293.89}]}'
```

**Output:**
```json
{
  "url": "https://api.example.com/v1/shipments/create.json",
  "method": "POST",
  "headers": [
    {"name": "Authorization", "value": "Token xxxx"},
    {"name": "Content-Type", "value": "application/json"}
  ],
  "route_params": [],
  "route_template": null,
  "query_params": [],
  "query_schema": null,
  "body": {
    "FormUrlencoded": [
      ["format", "json"],
      ["data", {"shipments": [{"name": "Shruti", "pin": "411021", "cod_amount": 1293.89}]}]
    ]
  },
  "body_schema": {
    "format": "string",
    "data": {
      "shipments": [{"name": "string", "pin": "string(numeric)", "cod_amount": "float"}]
    }
  },
  "auth": null,
  "cookies": [],
  "follow_redirects": true,
  "max_redirects": null,
  "insecure": false,
  "compressed": false,
  "connect_timeout": null,
  "max_time": null,
  "proxy": null,
  "user_agent": null,
  "referer": null,
  "output": null
}
```

---

Now parse the following curl command:

```
{{CURL_INPUT}}
```
