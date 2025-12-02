//! Tests for the EventTarget implementation.

use crate::test::{TestAction, run_test_actions_with};
use boa_engine::{Context, js_string};

/// Helper to run tests with Event and EventTarget registered
fn run_event_target_tests(actions: impl IntoIterator<Item = TestAction>) {
    let context = &mut Context::default();
    crate::event::register(context).expect("failed to register Event");
    crate::event::target::register(context).expect("failed to register EventTarget");
    run_test_actions_with(actions, context);
}

#[test]
fn test_event_target_constructor() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::assert("target instanceof EventTarget"),
    ]);
}

#[test]
fn test_add_event_listener_basic() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let called = false;"),
        TestAction::run("target.addEventListener('test', () => { called = true; });"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("called", true),
    ]);
}

#[test]
fn test_add_event_listener_multiple() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run("target.addEventListener('test', () => { count++; });"),
        TestAction::run("target.addEventListener('test', () => { count++; });"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("count", 2),
    ]);
}

#[test]
fn test_add_event_listener_duplicate_ignored() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run("const handler = () => { count++; };"),
        TestAction::run("target.addEventListener('test', handler);"),
        TestAction::run("target.addEventListener('test', handler);"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("count", 1),
    ]);
}

#[test]
fn test_add_event_listener_capture_option() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run("const handler = () => { count++; };"),
        TestAction::run("target.addEventListener('test', handler, { capture: true });"),
        TestAction::run("target.addEventListener('test', handler, { capture: false });"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("count", 2),
    ]);
}

#[test]
fn test_add_event_listener_once_option() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run("target.addEventListener('test', () => { count++; }, { once: true });"),
        TestAction::run("const event1 = new Event('test');"),
        TestAction::run("target.dispatchEvent(event1);"),
        TestAction::assert_eq("count", 1),
        TestAction::run("const event2 = new Event('test');"),
        TestAction::run("target.dispatchEvent(event2);"),
        TestAction::assert_eq("count", 1),
    ]);
}

#[test]
fn test_add_event_listener_passive_option() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let preventDefaultCalled = false;"),
        TestAction::run(
            "target.addEventListener('test', (e) => { e.preventDefault(); preventDefaultCalled = e.defaultPrevented; }, { passive: true });",
        ),
        TestAction::run("const event = new Event('test', { cancelable: true });"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("preventDefaultCalled", false),
    ]);
}

#[test]
fn test_add_event_listener_legacy_boolean_capture() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run("const handler = () => { count++; };"),
        TestAction::run("target.addEventListener('test', handler, true);"),
        TestAction::run("target.addEventListener('test', handler, false);"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("count", 2),
    ]);
}

#[test]
fn test_remove_event_listener_basic() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run("const handler = () => { count++; };"),
        TestAction::run("target.addEventListener('test', handler);"),
        TestAction::run("target.removeEventListener('test', handler);"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("count", 0),
    ]);
}

#[test]
fn test_remove_event_listener_nonexistent() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run("target.addEventListener('test', () => { count++; });"),
        TestAction::run("target.removeEventListener('test', () => {});"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("count", 1),
    ]);
}

#[test]
fn test_remove_event_listener_capture_mismatch() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run("const handler = () => { count++; };"),
        TestAction::run("target.addEventListener('test', handler, { capture: true });"),
        TestAction::run("target.removeEventListener('test', handler, { capture: false });"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("count", 1),
    ]);
}

#[test]
fn test_dispatch_event_returns_value() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("const event = new Event('test', { cancelable: true });"),
        TestAction::run("const result = target.dispatchEvent(event);"),
        TestAction::assert_eq("result", true),
    ]);
}

#[test]
fn test_dispatch_event_with_prevent_default() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("target.addEventListener('test', (e) => { e.preventDefault(); });"),
        TestAction::run("const event = new Event('test', { cancelable: true });"),
        TestAction::run("const result = target.dispatchEvent(event);"),
        TestAction::assert_eq("result", false),
    ]);
}

#[test]
fn test_dispatch_event_stop_propagation() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run(
            "target.addEventListener('test', (e) => { count++; e.stopPropagation(); });",
        ),
        TestAction::run("target.addEventListener('test', () => { count++; });"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("count", 2),
    ]);
}

#[test]
fn test_dispatch_event_stop_immediate_propagation() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let count = 0;"),
        TestAction::run(
            "target.addEventListener('test', (e) => { count++; e.stopImmediatePropagation(); });",
        ),
        TestAction::run("target.addEventListener('test', () => { count++; });"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("count", 1),
    ]);
}

#[test]
fn test_dispatch_event_different_types() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let testCalled = false;"),
        TestAction::run("let otherCalled = false;"),
        TestAction::run("target.addEventListener('test', () => { testCalled = true; });"),
        TestAction::run("target.addEventListener('other', () => { otherCalled = true; });"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert_eq("testCalled", true),
        TestAction::assert_eq("otherCalled", false),
    ]);
}

#[test]
fn test_event_target_receives_event_parameter() {
    run_event_target_tests([
        TestAction::run("const target = new EventTarget();"),
        TestAction::run("let receivedEvent = null;"),
        TestAction::run("target.addEventListener('test', (e) => { receivedEvent = e; });"),
        TestAction::run("const event = new Event('test');"),
        TestAction::run("target.dispatchEvent(event);"),
        TestAction::assert("receivedEvent !== null"),
        TestAction::assert_eq("receivedEvent.type", js_string!("test")),
    ]);
}
