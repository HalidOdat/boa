//! Tests for the DOMException implementation.

use crate::dom_exception::DOMException;
use crate::test::TestAction;
use crate::test::run_test_actions_with;
use boa_engine::Context;
use boa_engine::js_string;

/// Helper to run tests with DOMException registered
fn run_dom_exception_tests(actions: impl IntoIterator<Item = TestAction>) {
    let context = &mut Context::default();
    crate::dom_exception::register(context).expect("failed to register DOMException");
    run_test_actions_with(actions, context);
}

#[test]
fn test_dom_exception_constructor_defaults() {
    run_dom_exception_tests([
        TestAction::run("const e = new DOMException();"),
        TestAction::assert_eq("e.message", js_string!()),
        TestAction::assert_eq("e.name", js_string!("Error")),
        TestAction::assert_eq("e.code", 0),
    ]);
}

#[test]
fn test_dom_exception_constructor_with_message() {
    run_dom_exception_tests([
        TestAction::run("const e = new DOMException('Test message');"),
        TestAction::assert_eq("e.message", js_string!("Test message")),
        TestAction::assert_eq("e.name", js_string!("Error")),
        TestAction::assert_eq("e.code", 0),
    ]);
}

#[test]
fn test_dom_exception_constructor_with_name() {
    run_dom_exception_tests([
        TestAction::run("const e = new DOMException('Test message', 'NetworkError');"),
        TestAction::assert_eq("e.message", js_string!("Test message")),
        TestAction::assert_eq("e.name", js_string!("NetworkError")),
        TestAction::assert_eq("e.code", 19),
    ]);
}

#[test]
fn test_dom_exception_inherits_from_error() {
    run_dom_exception_tests([
        TestAction::assert("new DOMException() instanceof Error"),
        TestAction::assert("new DOMException() instanceof DOMException"),
    ]);
}

#[test]
fn test_dom_exception_all_legacy_codes() {
    run_dom_exception_tests([
        TestAction::assert_eq("new DOMException('test', 'IndexSizeError').code", 1),
        TestAction::assert_eq("new DOMException('test', 'HierarchyRequestError').code", 3),
        TestAction::assert_eq("new DOMException('test', 'WrongDocumentError').code", 4),
        TestAction::assert_eq("new DOMException('test', 'InvalidCharacterError').code", 5),
        TestAction::assert_eq(
            "new DOMException('test', 'NoModificationAllowedError').code",
            7,
        ),
        TestAction::assert_eq("new DOMException('test', 'NotFoundError').code", 8),
        TestAction::assert_eq("new DOMException('test', 'NotSupportedError').code", 9),
        TestAction::assert_eq("new DOMException('test', 'InUseAttributeError').code", 10),
        TestAction::assert_eq("new DOMException('test', 'InvalidStateError').code", 11),
        TestAction::assert_eq("new DOMException('test', 'SyntaxError').code", 12),
        TestAction::assert_eq(
            "new DOMException('test', 'InvalidModificationError').code",
            13,
        ),
        TestAction::assert_eq("new DOMException('test', 'NamespaceError').code", 14),
        TestAction::assert_eq("new DOMException('test', 'InvalidAccessError').code", 15),
        TestAction::assert_eq("new DOMException('test', 'TypeMismatchError').code", 17),
        TestAction::assert_eq("new DOMException('test', 'SecurityError').code", 18),
        TestAction::assert_eq("new DOMException('test', 'NetworkError').code", 19),
        TestAction::assert_eq("new DOMException('test', 'AbortError').code", 20),
        TestAction::assert_eq("new DOMException('test', 'URLMismatchError').code", 21),
        TestAction::assert_eq("new DOMException('test', 'QuotaExceededError').code", 22),
        TestAction::assert_eq("new DOMException('test', 'TimeoutError').code", 23),
        TestAction::assert_eq("new DOMException('test', 'InvalidNodeTypeError').code", 24),
        TestAction::assert_eq("new DOMException('test', 'DataCloneError').code", 25),
    ]);
}

#[test]
fn test_dom_exception_unknown_name_returns_zero() {
    run_dom_exception_tests([
        TestAction::run("const e = new DOMException('test', 'UnknownError');"),
        TestAction::assert_eq("e.code", 0),
        TestAction::assert_eq("e.name", js_string!("UnknownError")),
    ]);
}

#[test]
fn test_dom_exception_constants() {
    run_dom_exception_tests([
        TestAction::assert_eq("DOMException.INDEX_SIZE_ERR", 1),
        TestAction::assert_eq("DOMException.HIERARCHY_REQUEST_ERR", 3),
        TestAction::assert_eq("DOMException.WRONG_DOCUMENT_ERR", 4),
        TestAction::assert_eq("DOMException.INVALID_CHARACTER_ERR", 5),
        TestAction::assert_eq("DOMException.NO_MODIFICATION_ALLOWED_ERR", 7),
        TestAction::assert_eq("DOMException.NOT_FOUND_ERR", 8),
        TestAction::assert_eq("DOMException.NOT_SUPPORTED_ERR", 9),
        TestAction::assert_eq("DOMException.INUSE_ATTRIBUTE_ERR", 10),
        TestAction::assert_eq("DOMException.INVALID_STATE_ERR", 11),
        TestAction::assert_eq("DOMException.SYNTAX_ERR", 12),
        TestAction::assert_eq("DOMException.INVALID_MODIFICATION_ERR", 13),
        TestAction::assert_eq("DOMException.NAMESPACE_ERR", 14),
        TestAction::assert_eq("DOMException.INVALID_ACCESS_ERR", 15),
        TestAction::assert_eq("DOMException.TYPE_MISMATCH_ERR", 17),
        TestAction::assert_eq("DOMException.SECURITY_ERR", 18),
        TestAction::assert_eq("DOMException.NETWORK_ERR", 19),
        TestAction::assert_eq("DOMException.ABORT_ERR", 20),
        TestAction::assert_eq("DOMException.URL_MISMATCH_ERR", 21),
        TestAction::assert_eq("DOMException.QUOTA_EXCEEDED_ERR", 22),
        TestAction::assert_eq("DOMException.TIMEOUT_ERR", 23),
        TestAction::assert_eq("DOMException.INVALID_NODE_TYPE_ERR", 24),
        TestAction::assert_eq("DOMException.DATA_CLONE_ERR", 25),
    ]);
}

#[test]
fn test_dom_exception_constants_are_enumerable() {
    run_dom_exception_tests([
        TestAction::assert("Object.keys(DOMException).includes('INDEX_SIZE_ERR')"),
        TestAction::assert("Object.keys(DOMException).includes('NETWORK_ERR')"),
        TestAction::assert("Object.keys(DOMException).includes('DATA_CLONE_ERR')"),
    ]);
}

#[test]
fn test_dom_exception_properties_are_readonly() {
    run_dom_exception_tests([
        TestAction::run(
            r#"
            const e = new DOMException('msg', 'NetworkError');
            const originalMessage = e.message;
            const originalName = e.name;
            const originalCode = e.code;
            
            e.message = 'modified';
            e.name = 'modified';
            e.code = 999;
        "#,
        ),
        TestAction::assert_eq("e.message", js_string!("msg")),
        TestAction::assert_eq("e.name", js_string!("NetworkError")),
        TestAction::assert_eq("e.code", 19),
    ]);
}

#[test]
fn test_dom_exception_code_method() {
    let exception = DOMException {
        message: js_string!("test"),
        name: js_string!("NetworkError"),
    };

    assert_eq!(exception.code(), 19);

    let exception2 = DOMException {
        message: js_string!("test"),
        name: js_string!("CustomError"),
    };

    assert_eq!(exception2.code(), 0);
}
