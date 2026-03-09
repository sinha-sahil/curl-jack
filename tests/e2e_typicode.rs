use curl_jack::{parse, parse_response, Body, CurlRequest};

/// Helper: print a divider with a title
fn step(n: u8, title: &str) {
    let line = "-".repeat(60);
    println!("\n{line}");
    println!("  Step {n}: {title}");
    println!("{line}\n");
}

#[tokio::test]
async fn e2e_post_user_template_hydrate_execute() {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Step 1: Parse an original curl command (the "template source")
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    let curl_input = r#"curl -X POST https://jsonplaceholder.typicode.com/users \
        -H 'Content-Type: application/json' \
        -d '{"name":"Leanne Graham","username":"Bret","email":"Sincere@april.biz","phone":"1-770-736-8031","website":"hildegard.org"}'
    "#;

    step(1, "Parse the original curl command");

    let template = parse(curl_input).expect("failed to parse curl");
    print!("{template}");

    // Verify the template was parsed correctly
    assert_eq!(template.method, curl_jack::HttpMethod::POST);
    assert_eq!(
        template.url,
        "https://jsonplaceholder.typicode.com/users"
    );
    assert!(matches!(template.body, Some(Body::Json(_))));
    assert!(template.body_schema.is_some());

    println!("Body Schema (JSON):");
    if let Some(schema) = &template.body_schema {
        println!(
            "{}",
            serde_json::to_string_pretty(schema).unwrap_or_default()
        );
    }

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Step 2: Serialize the template to JSON (portable storage)
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    step(2, "Serialize template to JSON (portable format)");

    let template_json = serde_json::to_string_pretty(&template).expect("serialize failed");
    println!("{template_json}");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Step 3: Deserialize and hydrate with new data
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    step(3, "Hydrate the template with new user data");

    let mut hydrated: CurlRequest =
        serde_json::from_str(&template_json).expect("deserialize failed");

    // Replace the body with new user data, keeping everything else
    let new_user = serde_json::json!({
        "name": "Sahil Sinha",
        "username": "sahil.sinha",
        "email": "sahil@curljack.dev",
        "phone": "555-0199",
        "website": "curljack.dev"
    });
    hydrated.body = Some(Body::Json(new_user));

    print!("{hydrated}");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Step 4: Execute the hydrated request
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    step(4, "Execute the hydrated request");

    let raw_response = curl_jack::execute(&hydrated)
        .await
        .expect("request failed");

    println!("Raw status: {}", raw_response.status);
    println!("Raw body bytes: {}", raw_response.body.len());

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Step 5: Parse the response into structured form
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    step(5, "Parse the response");

    let parsed = parse_response(&raw_response);
    print!("{parsed}");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Assertions
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    assert_eq!(parsed.status, 201, "typicode should return 201 Created");
    assert_eq!(parsed.status_text, "Created");
    assert_eq!(
        parsed.status_class,
        curl_jack::StatusClass::Success
    );

    // The response body should be JSON with our data echoed back + an assigned id
    if let curl_jack::ResponseBody::Json(ref body) = parsed.body {
        assert_eq!(body["name"], "Sahil Sinha");
        assert_eq!(body["username"], "sahil.sinha");
        assert_eq!(body["email"], "sahil@curljack.dev");
        assert!(body["id"].is_number(), "typicode should assign an id");
        println!("\nAssigned id: {}", body["id"]);
    } else {
        panic!("expected JSON response body");
    }

    // Body schema should exist for JSON responses
    assert!(parsed.body_schema.is_some());

    println!("\nAll assertions passed.");
}

#[tokio::test]
async fn e2e_get_user_route_param_hydrate() {
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Step 1: Parse a GET curl with a route parameter
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    step(1, "Parse GET with route parameter");

    let curl_input = "curl https://jsonplaceholder.typicode.com/users/1";
    let template = parse(curl_input).expect("failed to parse");
    print!("{template}");

    // Should detect :id route param
    assert_eq!(template.route_params.len(), 1);
    assert_eq!(template.route_params[0].segment, "1");
    assert_eq!(
        template.route_params[0].kind,
        curl_jack::RouteParamKind::Integer
    );
    assert_eq!(
        template.route_template.as_deref(),
        Some("https://jsonplaceholder.typicode.com/users/:id")
    );

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Step 2: Hydrate — swap the route parameter to fetch user 3
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    step(2, "Hydrate: replace user id 1 -> 3");

    let mut hydrated = template.clone();
    hydrated.url = hydrated.url.replace("/1", "/3");
    print!("{hydrated}");

    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
    // Step 3: Execute and parse response
    // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

    step(3, "Execute and parse response");

    let raw = curl_jack::execute(&hydrated).await.expect("request failed");
    let parsed = parse_response(&raw);
    print!("{parsed}");

    assert_eq!(parsed.status, 200);

    if let curl_jack::ResponseBody::Json(ref body) = parsed.body {
        assert_eq!(body["id"], 3, "should get user with id 3");
        assert!(body["name"].is_string());
        println!("\nFetched user: {} (id={})", body["name"], body["id"]);
    } else {
        panic!("expected JSON response body");
    }

    assert!(parsed.body_schema.is_some());

    println!("\nAll assertions passed.");
}
