use std::fmt;

use super::auth::Auth;
use super::body::{Body, FormValue};
use super::request::CurlRequest;
use super::response::{ParsedResponse, ResponseBody, ResponseCookie};

impl fmt::Display for CurlRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ── Method + URL ──
        let display_url = self.route_template.as_deref().unwrap_or(&self.url);
        writeln!(f, "{} {}", self.method, display_url)?;
        if self.route_template.is_some() {
            writeln!(f, "  {}", self.url)?;
        }

        // ── Headers ──
        if !self.headers.is_empty() {
            writeln!(f)?;
            section(f, "Headers")?;
            let max = self.headers.iter().map(|h| h.name.len()).max().unwrap_or(0);
            for h in &self.headers {
                writeln!(f, "  {:<w$} : {}", h.name, h.value, w = max)?;
            }
        }

        // ── Auth ──
        if let Some(auth) = &self.auth {
            writeln!(f)?;
            section(f, "Auth")?;
            match auth {
                Auth::Basic { username, password } => {
                    writeln!(f, "  Basic {username}:{password}")?;
                }
                Auth::Bearer(token) => {
                    writeln!(f, "  Bearer {token}")?;
                }
            }
        }

        // ── Route Parameters ──
        if !self.route_params.is_empty() {
            writeln!(f)?;
            section(f, "Route Parameters")?;
            for p in &self.route_params {
                writeln!(f, "  {} = {}  (position {})", p.kind, p.segment, p.position)?;
            }
        }

        // ── Query Parameters ──
        if !self.query_params.is_empty() {
            writeln!(f)?;
            section(f, "Query Parameters")?;
            let max_key = self.query_params.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
            let schema_obj = self.query_schema.as_ref().and_then(|s| s.as_object());
            for (k, v) in &self.query_params {
                let type_hint = schema_obj
                    .and_then(|obj| obj.get(k))
                    .map(schema_leaf)
                    .unwrap_or_default();
                if type_hint.is_empty() {
                    writeln!(f, "  {:<w$} = {}", k, pretty_value(v), w = max_key)?;
                } else {
                    writeln!(
                        f,
                        "  {:<w$} = {:<vw$}  {type_hint}",
                        k,
                        pretty_value(v),
                        w = max_key,
                        vw = 16
                    )?;
                }
            }
        }

        // ── Body ──
        if let Some(body) = &self.body {
            writeln!(f)?;
            match body {
                Body::Json(v) => {
                    section(f, "Body (JSON)")?;
                    let pretty = serde_json::to_string_pretty(v).unwrap_or_default();
                    for line in pretty.lines() {
                        writeln!(f, "  {line}")?;
                    }
                }
                Body::FormUrlencoded(pairs) => {
                    section(f, "Body (Form URL-Encoded)")?;
                    let max_key = pairs.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
                    for (k, v) in pairs {
                        let val = pretty_value(v);
                        if val.len() > 60 {
                            writeln!(f, "  {k} =")?;
                            let pretty = serde_json::to_string_pretty(v).unwrap_or(val);
                            for line in pretty.lines() {
                                writeln!(f, "    {line}")?;
                            }
                        } else {
                            writeln!(f, "  {:<w$} = {val}", k, w = max_key)?;
                        }
                    }
                }
                Body::Raw(s) => {
                    section(f, "Body (Raw)")?;
                    for line in s.lines() {
                        writeln!(f, "  {line}")?;
                    }
                }
                Body::Binary(data) => {
                    section(f, "Body (Binary)")?;
                    writeln!(f, "  {len} bytes", len = data.len())?;
                }
                Body::Multipart(fields) => {
                    section(f, "Body (Multipart)")?;
                    let max_name = fields.iter().map(|ff| ff.name.len()).max().unwrap_or(0);
                    for field in fields {
                        match &field.value {
                            FormValue::Text(t) => {
                                writeln!(f, "  {:<w$} = {t}", field.name, w = max_name)?;
                            }
                            FormValue::File { path, mime } => {
                                let mime_str =
                                    mime.as_deref().map(|m| format!("  ({m})")).unwrap_or_default();
                                writeln!(
                                    f,
                                    "  {:<w$} = @{path}{mime_str}",
                                    field.name,
                                    w = max_name
                                )?;
                            }
                        }
                    }
                }
            }

            // ── Body Schema (flattened) ──
            if let Some(schema) = &self.body_schema {
                writeln!(f)?;
                section(f, "Body Schema")?;
                let mut flat = Vec::new();
                flatten_schema(schema, "", &mut flat);
                let max_path = flat.iter().map(|(p, _)| p.len()).max().unwrap_or(0);
                for (path, ty) in &flat {
                    writeln!(f, "  {:<w$} : {ty}", path, w = max_path)?;
                }
            }
        }

        // ── Cookies ──
        if !self.cookies.is_empty() {
            writeln!(f)?;
            section(f, "Cookies")?;
            let max = self.cookies.iter().map(|c| c.name.len()).max().unwrap_or(0);
            for c in &self.cookies {
                writeln!(f, "  {:<w$} = {}", c.name, c.value, w = max)?;
            }
        }

        // ── Options (only non-defaults) ──
        let mut opts: Vec<(&str, String)> = Vec::new();
        if self.follow_redirects {
            let val = match self.max_redirects {
                Some(n) => format!("yes (max {n})"),
                None => "yes".to_string(),
            };
            opts.push(("Follow redirects", val));
        }
        if self.insecure {
            opts.push(("Insecure (skip TLS)", "yes".to_string()));
        }
        if self.compressed {
            opts.push(("Compressed", "yes".to_string()));
        }
        if let Some(t) = &self.connect_timeout {
            opts.push(("Connect timeout", format_duration(t)));
        }
        if let Some(t) = &self.max_time {
            opts.push(("Max time", format_duration(t)));
        }
        if let Some(proxy) = &self.proxy {
            opts.push(("Proxy", proxy.clone()));
        }
        if let Some(ua) = &self.user_agent {
            opts.push(("User-Agent", ua.clone()));
        }
        if let Some(referer) = &self.referer {
            opts.push(("Referer", referer.clone()));
        }
        if let Some(output) = &self.output {
            opts.push(("Output", output.clone()));
        }

        if !opts.is_empty() {
            writeln!(f)?;
            section(f, "Options")?;
            let max_label = opts.iter().map(|(l, _)| l.len()).max().unwrap_or(0);
            for (label, val) in &opts {
                writeln!(f, "  {:<w$} : {val}", label, w = max_label)?;
            }
        }

        Ok(())
    }
}

fn section(f: &mut fmt::Formatter<'_>, title: &str) -> fmt::Result {
    writeln!(f, "{title}")
}

fn pretty_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => format!("\"{s}\""),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn schema_leaf(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        _ => String::new(),
    }
}

fn flatten_schema(v: &serde_json::Value, prefix: &str, out: &mut Vec<(String, String)>) {
    match v {
        serde_json::Value::String(s) => {
            out.push((prefix.to_string(), s.clone()));
        }
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for k in keys {
                let path = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_schema(&map[k], &path, out);
            }
        }
        serde_json::Value::Array(arr) => {
            if let Some(first) = arr.first() {
                let path = format!("{prefix}[]");
                flatten_schema(first, &path, out);
            } else {
                out.push((format!("{prefix}[]"), "empty".to_string()));
            }
        }
        _ => {
            out.push((prefix.to_string(), v.to_string()));
        }
    }
}

impl fmt::Display for ParsedResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // ── Status line ──
        writeln!(
            f,
            "{} {} ({})  <- {}",
            self.status,
            self.status_text,
            self.status_class,
            format_duration(&self.elapsed)
        )?;

        // ── Headers ──
        if !self.headers.is_empty() {
            writeln!(f)?;
            section(f, "Headers")?;
            let max = self.headers.iter().map(|h| h.name.len()).max().unwrap_or(0);
            for h in &self.headers {
                // Skip set-cookie from header list since we show them separately
                if h.name.eq_ignore_ascii_case("set-cookie") {
                    continue;
                }
                writeln!(f, "  {:<w$} : {}", h.name, h.value, w = max)?;
            }
        }

        // ── Cookies (Set-Cookie) ──
        if !self.cookies.is_empty() {
            writeln!(f)?;
            section(f, "Cookies (Set-Cookie)")?;
            let max_name = self.cookies.iter().map(|c| c.name.len()).max().unwrap_or(0);
            for c in &self.cookies {
                let attrs = format_cookie_attrs(c);
                if attrs.is_empty() {
                    writeln!(f, "  {:<w$} = {}", c.name, c.value, w = max_name)?;
                } else {
                    writeln!(
                        f,
                        "  {:<w$} = {}  ({})",
                        c.name, c.value, attrs, w = max_name
                    )?;
                }
            }
        }

        // ── Body ──
        writeln!(f)?;
        match &self.body {
            ResponseBody::Json(v) => {
                section(f, "Body (JSON)")?;
                let pretty = serde_json::to_string_pretty(v).unwrap_or_default();
                for line in pretty.lines() {
                    writeln!(f, "  {line}")?;
                }
            }
            ResponseBody::Html(text) => {
                section(f, "Body (HTML)")?;
                write_body_preview(f, text, 20)?;
            }
            ResponseBody::Xml(text) => {
                section(f, "Body (XML)")?;
                write_body_preview(f, text, 20)?;
            }
            ResponseBody::Text(text) => {
                section(f, "Body (Text)")?;
                write_body_preview(f, text, 20)?;
            }
            ResponseBody::Binary { size } => {
                section(f, "Body (Binary)")?;
                writeln!(f, "  {size} bytes")?;
            }
            ResponseBody::Empty => {
                section(f, "Body")?;
                writeln!(f, "  (empty)")?;
            }
        }

        // ── Body Schema (flattened) ──
        if let Some(schema) = &self.body_schema {
            writeln!(f)?;
            section(f, "Body Schema")?;
            let mut flat = Vec::new();
            flatten_schema(schema, "", &mut flat);
            let max_path = flat.iter().map(|(p, _)| p.len()).max().unwrap_or(0);
            for (path, ty) in &flat {
                writeln!(f, "  {:<w$} : {ty}", path, w = max_path)?;
            }
        }

        Ok(())
    }
}

fn format_cookie_attrs(c: &ResponseCookie) -> String {
    let mut parts = Vec::new();
    if let Some(path) = &c.path {
        parts.push(format!("path={path}"));
    }
    if let Some(domain) = &c.domain {
        parts.push(format!("domain={domain}"));
    }
    if let Some(max_age) = c.max_age {
        parts.push(format!("max-age={max_age}"));
    }
    if let Some(expires) = &c.expires {
        parts.push(format!("expires={expires}"));
    }
    if c.http_only {
        parts.push("HttpOnly".to_string());
    }
    if c.secure {
        parts.push("Secure".to_string());
    }
    if let Some(ss) = &c.same_site {
        parts.push(format!("SameSite={ss}"));
    }
    parts.join("; ")
}

fn write_body_preview(f: &mut fmt::Formatter<'_>, text: &str, max_lines: usize) -> fmt::Result {
    let lines: Vec<&str> = text.lines().collect();
    let show = lines.len().min(max_lines);
    for line in &lines[..show] {
        writeln!(f, "  {line}")?;
    }
    if lines.len() > max_lines {
        writeln!(f, "  ... ({} more lines)", lines.len() - max_lines)?;
    }
    Ok(())
}

fn format_duration(d: &std::time::Duration) -> String {
    let millis = d.as_millis();
    if millis < 1000 {
        format!("{millis}ms")
    } else {
        let secs = d.as_secs_f64();
        if secs == secs.floor() {
            format!("{}s", secs as u64)
        } else {
            format!("{secs:.2}s")
        }
    }
}
