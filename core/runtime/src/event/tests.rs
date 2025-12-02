//! Tests for the Event implementation.

use crate::test::TestAction;
use crate::test::run_test_actions_with;
use boa_engine::Context;
use boa_engine::js_string;

/// Helper to run tests with Event registered
fn run_event_tests(actions: impl IntoIterator<Item = TestAction>) {
    let context = &mut Context::default();
    crate::event::register(context).expect("failed to register Event");
    crate::event::target::register(context).expect("failed to register EventTarget");
    run_test_actions_with(actions, context);
}

#[test]
fn test_event_constructor_minimal() {
    run_event_tests([
        TestAction::run("const e = new Event('test');"),
        TestAction::assert_eq("e.type", js_string!("test")),
        TestAction::assert_eq("e.bubbles", false),
        TestAction::assert_eq("e.cancelable", false),
        TestAction::assert_eq("e.composed", false),
    ]);
}

#[test]
fn test_event_constructor_with_options() {
    run_event_tests([
        TestAction::run(
            "const e = new Event('test', { bubbles: true, cancelable: true, composed: true });",
        ),
        TestAction::assert_eq("e.type", js_string!("test")),
        TestAction::assert_eq("e.bubbles", true),
        TestAction::assert_eq("e.cancelable", true),
        TestAction::assert_eq("e.composed", true),
    ]);
}

#[test]
fn test_event_default_prevented() {
    run_event_tests([
        TestAction::run("const e = new Event('test', { cancelable: true });"),
        TestAction::assert_eq("e.defaultPrevented", false),
        TestAction::run("e.preventDefault();"),
        TestAction::assert_eq("e.defaultPrevented", true),
    ]);
}

#[test]
fn test_event_prevent_default_non_cancelable() {
    run_event_tests([
        TestAction::run("const e = new Event('test');"),
        TestAction::run("e.preventDefault();"),
        TestAction::assert_eq("e.defaultPrevented", false),
    ]);
}

#[test]
fn test_event_stop_propagation() {
    run_event_tests([
        TestAction::run("const e = new Event('test');"),
        TestAction::run("e.stopPropagation();"),
    ]);
}

#[test]
fn test_event_stop_immediate_propagation() {
    run_event_tests([
        TestAction::run("const e = new Event('test');"),
        TestAction::run("e.stopImmediatePropagation();"),
    ]);
}

#[test]
fn test_event_is_trusted() {
    run_event_tests([
        TestAction::run("const e = new Event('test');"),
        TestAction::assert_eq("e.isTrusted", false),
    ]);
}

#[test]
fn test_event_event_phase() {
    run_event_tests([
        TestAction::run("const e = new Event('test');"),
        TestAction::assert_eq("e.eventPhase", 0),
    ]);
}

#[test]
fn test_event_target() {
    run_event_tests([
        TestAction::run("const e = new Event('test');"),
        TestAction::assert("e.target === null"),
    ]);
}

#[test]
fn test_event_time_stamp() {
    run_event_tests([
        TestAction::run("const e = new Event('test');"),
        TestAction::assert("typeof e.timeStamp === 'number'"),
        TestAction::assert("e.timeStamp >= 0"),
    ]);
}

#[test]
fn test_event_constants() {
    run_event_tests([
        TestAction::assert_eq("Event.NONE", 0),
        TestAction::assert_eq("Event.CAPTURING_PHASE", 1),
        TestAction::assert_eq("Event.AT_TARGET", 2),
        TestAction::assert_eq("Event.BUBBLING_PHASE", 3),
    ]);
}

#[test]
fn test_event_constants_are_enumerable() {
    run_event_tests([
        TestAction::assert("Object.keys(Event).includes('NONE')"),
        TestAction::assert("Object.keys(Event).includes('CAPTURING_PHASE')"),
        TestAction::assert("Object.keys(Event).includes('AT_TARGET')"),
        TestAction::assert("Object.keys(Event).includes('BUBBLING_PHASE')"),
    ]);
}

#[test]
fn test_event_init_event() {
    run_event_tests([
        TestAction::run("const e = new Event('original');"),
        TestAction::assert_eq("e.type", js_string!("original")),
        TestAction::assert_eq("e.bubbles", false),
        TestAction::assert_eq("e.cancelable", false),
        TestAction::run("e.initEvent('modified', true, true);"),
        TestAction::assert_eq("e.type", js_string!("modified")),
        TestAction::assert_eq("e.bubbles", true),
        TestAction::assert_eq("e.cancelable", true),
    ]);
}

#[test]
fn test_event_init_event_during_dispatch() {
    run_event_tests([
        TestAction::run(
            r#"
            const target = new EventTarget();
            let eventDuringDispatch = null;
            target.addEventListener('test', (e) => {
                eventDuringDispatch = e;
                e.initEvent('modified', true, true);
            });
            const evt = new Event('test');
            target.dispatchEvent(evt);
        "#,
        ),
        // initEvent should have no effect during dispatch
        TestAction::assert_eq("eventDuringDispatch.type", js_string!("test")),
        TestAction::assert_eq("eventDuringDispatch.bubbles", false),
    ]);
}

#[test]
fn test_event_properties_are_readonly() {
    run_event_tests([
        TestAction::run(
            r#"
            const e = new Event('test', { bubbles: true });
            e.type = 'modified';
            e.bubbles = false;
            e.eventPhase = 999;
        "#,
        ),
        TestAction::assert_eq("e.type", js_string!("test")),
        TestAction::assert_eq("e.bubbles", true),
        TestAction::assert_eq("e.eventPhase", 0),
    ]);
}
