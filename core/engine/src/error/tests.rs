use super::*;
use crate::{Context, Source};

#[test]
fn test_js_error_display_matches_erased() {
    let mut context = Context::default();

    // Create a simple error with a message
    let native_error = JsNativeError::typ().with_message("test error message");
    let js_error = JsError::from_native(native_error);

    // Get the display output of the original JsError
    let original_display = format!("{}", js_error);

    // Convert to erased and get its display output
    let erased = js_error.into_erased(&mut context);
    let erased_display = format!("{}", erased);

    // Both should contain the same error type and message
    assert!(original_display.contains("TypeError: test error message"));
    assert!(erased_display.contains("TypeError: test error message"));

    // The erased version loses position info from the error header,
    // so it won't have position information (since this error has no backtrace)
    // Now the erased error should include position info like the original
    println!("Original: {}", original_display);
    println!("Erased: {}", erased_display);
    assert_eq!(original_display, erased_display);
}

#[test]
fn test_js_error_with_backtrace_display_matches_erased() {
    let mut context = Context::default();

    // Execute JavaScript code that throws an error with a backtrace
    let code = r#"
        function foo() {
            throw new TypeError('Error in foo');
        }
        function bar() {
            foo();
        }
        bar();
    "#;

    let result = context.eval(Source::from_bytes(code));
    let js_error = result.unwrap_err();

    // Get the display output of the original JsError
    let original_display = format!("{}", js_error);

    // Convert to erased and get its display output
    let erased = js_error.into_erased(&mut context);
    let erased_display = format!("{}", erased);

    // The erased version won't include position info in the error message itself,
    // but it should be in the backtrace. Both should have the same backtrace.
    assert!(original_display.contains("TypeError: Error in foo"));
    assert!(erased_display.contains("TypeError: Error in foo"));

    // Verify that the backtrace is actually present
    assert!(original_display.contains("at foo"));
    assert!(original_display.contains("at bar"));
    assert!(erased_display.contains("at foo"));
    assert!(erased_display.contains("at bar"));

    // Verify that both have the same backtrace lines
    let original_lines: Vec<&str> = original_display.lines().skip(1).collect();
    let erased_lines: Vec<&str> = erased_display.lines().skip(1).collect();
    assert_eq!(
        original_lines, erased_lines,
        "Backtrace should be identical"
    );
}
