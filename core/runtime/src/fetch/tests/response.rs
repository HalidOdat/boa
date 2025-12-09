use super::TestFetcher;
use crate::test::{TestAction, run_test_actions};
use boa_engine::{Context, js_str};
use http::{Response, Uri};

fn register(responses: &[(&'static str, Response<Vec<u8>>)], ctx: &mut Context) {
    let mut fetcher = TestFetcher::default();

    for (url, resp) in responses {
        fetcher.add_response(Uri::from_static(url), resp.clone());
    }
    crate::fetch::register(fetcher, None, ctx).expect("failed to register fetch");
}

#[test]
fn response_error() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| register(&[], ctx)),
        TestAction::run(
            r#"
                const response = Response.error();

                assertEq(response.status, 0);
                assertEq(response.statusText, "");
                assertEq(response.headers.get("custom-header"), null);
                assertEq(response.type, "error");
                assertEq(response.url, "");
            "#,
        ),
        // Assertions made in JavaScript.
    ]);
}

#[test]
fn response_text() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| {
            register(
                &[("http://unit.test", Response::new(b"Hello World".to_vec()))],
                ctx,
            );
        }),
        TestAction::run(
            r#"
                globalThis.response = (async () => {
                    const request = new Request("http://unit.test");
                    const response = await fetch(request);
                    const text = await response.text();
                    assertEq(text, "Hello World");
                })();
            "#,
        ),
        TestAction::inspect_context(|ctx| {
            let response = ctx.global_object().get(js_str!("response"), ctx).unwrap();
            response.as_promise().unwrap().await_blocking(ctx).unwrap();
        }),
    ]);
}

#[test]
fn response_json() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| {
            register(
                &[(
                    "http://unit.test",
                    Response::new(b"{ \"hello world\": 123 }".to_vec()),
                )],
                ctx,
            );
        }),
        TestAction::run(
            r#"
                globalThis.response = (async () => {
                    const request = new Request("http://unit.test");
                    const response = await fetch(request);
                    const json = await response.json();
                    assertEq(json["hello world"], 123);
                    return json;
                })();
            "#,
        ),
        TestAction::inspect_context(|ctx| {
            let response = ctx.global_object().get(js_str!("response"), ctx).unwrap();
            let response = response.as_promise().unwrap().await_blocking(ctx).unwrap();
            assert_eq!(
                format!("{}", response.display_obj(false)),
                "{\n    hello world: 123\n}"
            );
        }),
    ]);
}

#[test]
fn response_bytes() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| {
            register(
                &[("http://unit.test", Response::new(b"Hello World".to_vec()))],
                ctx,
            );
        }),
        TestAction::run(
            r#"
                globalThis.response = (async () => {
                    const request = new Request("http://unit.test");
                    const response = await fetch(request);
                    const bytes = await response.bytes();
                    const text = new TextDecoder().decode(bytes);
                    assertEq(text, "Hello World");
                })();
            "#,
        ),
        TestAction::inspect_context(|ctx| {
            let response = ctx.global_object().get(js_str!("response"), ctx).unwrap();
            response.as_promise().unwrap().await_blocking(ctx).unwrap();
        }),
    ]);
}

#[test]
fn response_getter() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| {
            let mut response = Response::new(b"Hello World".to_vec());
            response
                .headers_mut()
                .append("custom-header", "custom-value".parse().unwrap());
            register(&[("http://unit.test", response)], ctx);
        }),
        TestAction::run(
            r#"
                globalThis.response = (async () => {
                    const request = new Request("http://unit.test");
                    const response = await fetch(request);

                    assertEq(response.status, 200);
                    assertEq(response.statusText, "OK");
                    assertEq(response.headers.get("custom-header"), "custom-value");
                    assertEq(response.type, "basic");
                    assertEq(response.url, "http://unit.test/");
                })();
            "#,
        ),
        TestAction::inspect_context(|ctx| {
            let response = ctx.global_object().get(js_str!("response"), ctx).unwrap();
            response.as_promise().unwrap().await_blocking(ctx).unwrap();

            // Assertions made in JavaScript.
        }),
    ]);
}

#[test]
fn response_constructor_basic() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| register(&[], ctx)),
        TestAction::run(
            r#"
                const response = new Response("Hello, World!");
                assertEq(response.status, 200);
                assertEq(response.statusText, "OK");
                assertEq(response.type, "basic");
            "#,
        ),
    ]);
}

#[test]
fn response_constructor_with_status() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| register(&[], ctx)),
        TestAction::run(
            r#"
                const response = new Response("Not Found", { status: 404 });
                assertEq(response.status, 404);
                assertEq(response.statusText, "Not Found");
            "#,
        ),
    ]);
}

#[test]
fn response_constructor_with_status_text() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| register(&[], ctx)),
        TestAction::run(
            r#"
                const response = new Response("OK", { status: 200, statusText: "Custom Status" });
                assertEq(response.status, 200);
                assertEq(response.statusText, "Custom Status");
            "#,
        ),
    ]);
}

#[test]
fn response_constructor_with_headers() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| register(&[], ctx)),
        TestAction::run(
            r#"
                const response = new Response("OK", { 
                    headers: { "Content-Type": "application/json" }
                });
                assertEq(response.headers.get("Content-Type"), "application/json");
            "#,
        ),
    ]);
}

#[test]
fn response_constructor_null_body() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| register(&[], ctx)),
        TestAction::run(
            r#"
                const response = new Response(null);
                assertEq(response.status, 200);
            "#,
        ),
    ]);
}

#[test]
fn response_constructor_invalid_status() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| register(&[], ctx)),
        TestAction::run(
            r#"
                try {
                    new Response("test", { status: 100 });
                    throw new Error("Should have thrown RangeError");
                } catch (e) {
                    if (e.name !== "RangeError") {
                        throw e;
                    }
                }
            "#,
        ),
    ]);
}

#[test]
fn response_constructor_null_body_status() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| register(&[], ctx)),
        TestAction::run(
            r#"
                try {
                    new Response("test", { status: 204 });
                    throw new Error("Should have thrown TypeError");
                } catch (e) {
                    if (e.name !== "TypeError") {
                        throw e;
                    }
                }
            "#,
        ),
    ]);
}

#[test]
fn response_constructor_body_text() {
    run_test_actions([
        TestAction::harness(),
        TestAction::inspect_context(|ctx| register(&[], ctx)),
        TestAction::run(
            r#"
                globalThis.response = (async () => {
                    const response = new Response("Hello, World!");
                    const text = await response.text();
                    assertEq(text, "Hello, World!");
                })();
            "#,
        ),
        TestAction::inspect_context(|ctx| {
            let response = ctx.global_object().get(js_str!("response"), ctx).unwrap();
            response.as_promise().unwrap().await_blocking(ctx).unwrap();
        }),
    ]);
}
