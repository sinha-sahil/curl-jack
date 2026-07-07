// cargo run --example jsonplaceholder_vars

use wire_jack::{run, Template, Value};

const CONTRACT: &str = r#"{{
    input.phase == "request"
      ? let base  = "https://jsonplaceholder.typicode.com" in
        let path  = concat("/posts/", to_string(input.request.id)) in
        let token = concat("Bearer ", input.request.token) in {
          "url":    concat(base, path),
          "method": "GET",
          "headers": {
            "Authorization": token,
            "X-Trace-Id":    input.request.trace,
            "Accept":        "application/json"
          }
        }
      : {
          "post_id": input.response.body_json.id,
          "author":  input.response.body_json.userId,
          "title":   upper(input.response.body_json.title),
          "ok":      input.response.ok
        }
}}"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let template = Template::compile(CONTRACT).map_err(|e| {
        e.iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;

    let data = Value::obj([
        ("id", Value::Int(3)),
        ("token", Value::Str("abc123".into())),
        ("trace", Value::Str("trace-9".into())),
    ]);
    let result = run(&template, data).await?;
    println!("{}", serde_json::to_string_pretty(&result.output)?);

    Ok(())
}
