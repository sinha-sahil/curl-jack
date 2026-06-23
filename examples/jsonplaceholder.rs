// cargo run --example jsonplaceholder

use wire_jack::{run, Template, Value};

const CONTRACT: &str = r#"{{
    input.phase == "request"
      ? {
          "url":    concat("https://jsonplaceholder.typicode.com/posts/", to_string(input.request.id)),
          "method": "GET"
        }
      : {
          "post_id": input.response.body_json.id,
          "author":  input.response.body_json.userId,
          "title":   upper(input.response.body_json.title),
          "status":  input.response.status,
          "ok":      input.response.ok
        }
}}"#;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let template = Template::compile(CONTRACT)
        .map_err(|e| e.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n"))?;

    for id in [1, 2] {
        let output = run(&template, Value::obj([("id", Value::Int(id))])).await?;
        println!("id={id} ->\n{}\n", serde_json::to_string_pretty(&output)?);
    }
    Ok(())
}
