use wire_jack::{execute, ApiRequest, Template, Value};

// Live network test; run with `cargo test -- --ignored`.
#[tokio::test]
#[ignore]
async fn render_and_execute_get() {
    let template = Template::compile(
        r#"{ "url": {{ concat("https://jsonplaceholder.typicode.com/users/", to_string(input.id)) }} }"#,
    )
    .unwrap();

    let req: ApiRequest = template.render(Value::obj([("id", Value::Int(1))])).unwrap();
    let resp = execute(&req).await.unwrap();

    assert_eq!(resp.status, 200);
    assert!(resp.ok);
    assert_eq!(resp.body_json.unwrap()["id"], serde_json::json!(1));
    assert!(resp.body_text.is_none());
}
