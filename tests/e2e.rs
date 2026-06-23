use wire_jack::{run, Template, Value};

// Live network test; run with `cargo test -- --ignored`.
#[tokio::test]
#[ignore]
async fn run_contract_live() {
    let template = Template::compile(
        r#"{{
            input.phase == "request"
              ? { "url": concat("https://jsonplaceholder.typicode.com/posts/", to_string(input.request.id)), "method": "GET" }
              : { "id": input.response.body_json.id, "ok": input.response.ok }
        }}"#,
    )
    .unwrap();

    let out = run(&template, Value::obj([("id", Value::Int(1))]))
        .await
        .unwrap();
    assert_eq!(out["id"], serde_json::json!(1));
    assert_eq!(out["ok"], serde_json::json!(true));
}
