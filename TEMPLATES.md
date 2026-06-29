# Writing wire-jack templates

A contract is **one** [temple-dsl](https://github.com/sinha-sahil/temple-dsl) template. `run` renders it twice — once to build the request, once to shape the response — so the template must handle both phases.

## 1. Skeleton

```jsonc
{{
  input.phase == "request"
    ? { …ApiRequest… }        # build the request from input.request.*
    : { …free-form output… }  # shape the response from input.request.* and input.response.*
}}
```

- The whole template is **one** scalar hole `{{ … }}` wrapping a single expression (a ternary, or `when`).
- Inside `{{ }}` you are in **expression context**: build objects as `{ "key": expr }` with **bare expressions**. Do **not** nest `{{ }}`, and build strings with `concat(…)` — string interpolation (`"… {{ x }} …"`) is *not* available here.
- The **request** branch output is deserialized into [`ApiRequest`](README.md#apirequest). The **response** branch output is returned as-is (free-form JSON).

## 2. Inputs per phase

| `input.phase` | Your data | Must produce |
|---|---|---|
| `"request"` | `input.request.*` (whatever you passed to `run`) | an `ApiRequest` object |
| `"response"` | `input.request.*` (the original data) and `input.response.*` (the [`ApiResponse`](README.md#apiresponse)) | any JSON shape |

- Reserved top-level keys are `phase`, `request`, `response`. Your data lives under `input.request` regardless of its type (object, string, number, array).
- In the request phase `input.response` does **not** exist. In the response phase `input.request` is still available, so response mapping can combine the original input with the API response.

## 3. The request object

Emit only the keys you need — everything else defaults.

```jsonc
{
  "url":     concat("https://api/", to_string(input.request.id)),  // required
  "method":  "POST",                                               // string, default "GET"
  "headers": { "Authorization": concat("Bearer ", input.request.token) },
  "query":   { "page": "1" },
  "cookies": { "session": input.request.sid },
  "body":    { "name": input.request.name },                       // see §4
  "auth":    { "scheme": "bearer", "token": input.request.token },
             // or { "scheme": "basic", "username": "u", "password": "p" }
  "timeout_ms": 5000,
  "follow_redirects": true, "max_redirects": 5,
  "proxy": "http://proxy:8080", "insecure": false,
  "retry": { "max_attempts": 3, "backoff": "exponential",
             "delay_ms": 200, "max_delay_ms": 5000, "retry_on_status": [429, 503] }
}
```

- `url` is the only required key; `method` is a plain **string** (not an enum).
- `headers`, `query`, `cookies` values must be **strings** — cast numbers with `to_string(…)`.

## 4. Body encoding

The request "type" is not a field — it is whatever `Content-Type` says. Set the header, shape `body` to match:

| `Content-Type` | `body` should be | Sent as |
|---|---|---|
| *absent* (body present) | object | `application/json` |
| `application/json` | object / array | JSON |
| `application/x-www-form-urlencoded` | object | `k=v&…` |
| `multipart/form-data` | object; a field `{ "$file": "/path" }` → file part, else text part | multipart |
| `application/octet-stream`, `image/*`, … | a **base64 string** | raw bytes (decoded) |
| `text/*`, `application/xml`, … | a string (sent verbatim) | raw text |

## 5. Variables

**Phase-scoped (preferred)** — a `let … in` *inside* a branch evaluates only when that branch is taken, so plain access is safe:

```jsonc
? let path  = concat("/users/", to_string(input.request.id)) in
  let token = concat("Bearer ", input.request.token) in {
    "url":     concat(input.request.base, path),
    "headers": { "Authorization": token }
  }
```

Chain bindings: `let a = … in let b = … in { … }`.

**Top-level `let`** evaluates in **both** phases — guard response-only reads with optional access from the first hop, or they error in the request phase:

```jsonc
let status = input?.response?.status   # null in the request phase, no error
```

`this.key` reads a sibling key of the object you are building.

## 6. temple syntax cheat-sheet

| | |
|---|---|
| **Access** | `a.b` · `a[0]` · `a["k"]` — plain `.` **errors** if the key is missing |
| **Safe access** | `a?.b` → `null` if absent · `a ?? b` → `b` when `a` is `null` |
| **Operators** | `+ - * / %` · `== != < <= > >=` · `&& \|\| !` · `cond ? a : b` |
| **Guards** | `when { cond: v, … , else: v }` |
| **Literals** | numbers · `'single'` or `"double"` quoted strings · `true` `false` `null` · `[a, b]` · `{ "k": v }` |
| **Object extras** | computed key `{ [expr]: v }` · omit-if-null `"k"?: v` |
| **Lambdas** | `x -> x * 2` · `(acc, x) -> acc + x` (array methods only) |
| **Comments** | `# to end of line` |

**Built-in functions:** `abs` `round` `floor` `ceil` `min` `max` · `upper` `lower` `trim` · `to_string` `to_number` · `concat` · `json_encode` `url_encode` `base64` · `type_of` `is_null` `is_bool` `is_number` `is_string` `is_array` `is_object`

**Methods** (chain on any value): arrays — `.map .filter .fold .sum .any .all .count .find .contains .sort .reverse .unique .flatten .join .min .max .avg .first .last .len()`; strings — `.contains .starts_with .ends_with .replace .split .slice .upper() .lower() .trim()`; objects — `.keys() .values() .has() .get() .merge()`.

## 7. Gotchas

- **One `{{ }}` only.** Wrap the whole template; inside, use bare expressions and `concat(…)` for strings — no nested `{{ }}`, no `"… {{ x }} …"` interpolation.
- **`method` is a string** — data-carrying enums don't render.
- **Decimals become JSON strings** in a body (exact, never `f64`); integers stay numbers. `to_number("19.99")` → `"19.99"` on the wire.
- **Cast before stringy fields** — `to_string(n)` for anything going into `url` / `headers` / `query`.
- **Don't read `input.response` in the request branch** — it only exists after the HTTP call. `input.request` is available in both branches.

## 8. Full example

```jsonc
{{
  input.phase == "request"
    ? let id = to_string(input.request.id) in {
        "url":     concat("https://api.example.com/posts/", id),
        "method":  "POST",
        "headers": { "Content-Type": "application/json",
                     "Authorization": concat("Bearer ", input.request.token) },
        "query":   { "draft": "false" },
        "body":    { "title": input.request.title, "views": input.request.views }
      }
    : {
        "post_id": input.response.body_json.id,
        "title":   upper(input.response.body_json.title),
        "ok":      input.response.ok,
        "took_ms": input.response.elapsed_ms
      }
}}
```
