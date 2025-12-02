//! Boa's implementation of the WHATWG DOM `DOMException` interface.
//!
//! The DOMException interface represents an error that occurs in DOM operations.
//!
//! More information:
//!  - [MDN documentation][mdn]
//!  - [WHATWG DOM specification][spec]
//!
//! [spec]: https://webidl.spec.whatwg.org/#idl-DOMException
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/DOMException

use boa_engine::string::JsStr;
use boa_engine::{
    Context, Finalize, JsData, JsObject, JsResult, JsString, Trace, js_str, js_string,
};

/// Legacy error codes for DOMException.
///
/// These numeric error codes are defined in the WebIDL specification for backwards compatibility.
///
/// [spec]: https://webidl.spec.whatwg.org/#idl-DOMException-error-names
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Trace, Finalize)]
#[boa_gc(unsafe_empty_trace)]
pub enum Code {
    /// Index or size is negative or greater than the allowed value.
    IndexSizeErr = 1,
    /// The object is in the wrong document.
    HierarchyRequestErr = 3,
    /// The object is in the wrong document.
    WrongDocumentErr = 4,
    /// The string contains invalid characters.
    InvalidCharacterErr = 5,
    /// The object cannot be modified.
    NoModificationAllowedErr = 7,
    /// The object cannot be found here.
    NotFoundErr = 8,
    /// The operation is not supported.
    NotSupportedErr = 9,
    /// The attribute is in use by another element.
    InuseAttributeErr = 10,
    /// The object is in an invalid state.
    InvalidStateErr = 11,
    /// The string did not match the expected pattern.
    SyntaxErr = 12,
    /// The object cannot be modified in this way.
    InvalidModificationErr = 13,
    /// The operation is not allowed by namespaces in XML.
    NamespaceErr = 14,
    /// The object does not support the operation or argument.
    InvalidAccessErr = 15,
    /// The type of the object does not match the expected type.
    TypeMismatchErr = 17,
    /// The operation is insecure.
    SecurityErr = 18,
    /// A network error occurred.
    NetworkErr = 19,
    /// The operation was aborted.
    AbortErr = 20,
    /// The given URL does not match another URL.
    UrlMismatchErr = 21,
    /// The quota has been exceeded.
    QuotaExceededErr = 22,
    /// The operation timed out.
    TimeoutErr = 23,
    /// The supplied node is incorrect or has an incorrect ancestor for this operation.
    InvalidNodeTypeErr = 24,
    /// The object cannot be cloned.
    DataCloneErr = 25,
}

impl Code {
    /// Convert the error code to its numeric representation.
    const fn code(self) -> u16 {
        self as u16
    }

    /// Mapping of error names to their corresponding error codes.
    const CODE_MAPPING: &[(JsStr<'static>, Code)] = &[
        (js_str!("IndexSizeError"), Self::IndexSizeErr),
        (js_str!("HierarchyRequestError"), Self::HierarchyRequestErr),
        (js_str!("WrongDocumentError"), Self::WrongDocumentErr),
        (js_str!("InvalidCharacterError"), Self::InvalidCharacterErr),
        (
            js_str!("NoModificationAllowedError"),
            Self::NoModificationAllowedErr,
        ),
        (js_str!("NotFoundError"), Self::NotFoundErr),
        (js_str!("NotSupportedError"), Self::NotSupportedErr),
        (js_str!("InUseAttributeError"), Self::InuseAttributeErr),
        (js_str!("InvalidStateError"), Self::InvalidStateErr),
        (js_str!("SyntaxError"), Self::SyntaxErr),
        (
            js_str!("InvalidModificationError"),
            Self::InvalidModificationErr,
        ),
        (js_str!("NamespaceError"), Self::NamespaceErr),
        (js_str!("InvalidAccessError"), Self::InvalidAccessErr),
        (js_str!("TypeMismatchError"), Self::TypeMismatchErr),
        (js_str!("SecurityError"), Self::SecurityErr),
        (js_str!("NetworkError"), Self::NetworkErr),
        (js_str!("AbortError"), Self::AbortErr),
        (js_str!("URLMismatchError"), Self::UrlMismatchErr),
        (js_str!("QuotaExceededError"), Self::QuotaExceededErr),
        (js_str!("TimeoutError"), Self::TimeoutErr),
        (js_str!("InvalidNodeTypeError"), Self::InvalidNodeTypeErr),
        (js_str!("DataCloneError"), Self::DataCloneErr),
    ];

    /// Convert a name string to its corresponding error code enum variant.
    ///
    /// Returns `Some(Code)` for known exception names, or `None` for unknown names.
    fn from_name(name: JsStr<'_>) -> Option<Self> {
        Self::CODE_MAPPING
            .iter()
            .find(|(error_name, _)| *error_name == name)
            .map(|(_, code)| *code)
    }
}

/// The [`DOMException`][mdn] interface represents an error that occurs in DOM operations.
///
/// [spec]: https://webidl.spec.whatwg.org/#idl-DOMException
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/DOMException
#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct DOMException {
    pub(crate) message: JsString,
    pub(crate) name: JsString,
}

#[boa_engine::boa_class]
impl DOMException {
    // Legacy error code constants
    /// Legacy error code for IndexSizeError (value: 1).
    #[boa(constant)]
    pub const INDEX_SIZE_ERR: u16 = Code::IndexSizeErr.code();

    /// Legacy error code for HierarchyRequestError (value: 3).
    #[boa(constant)]
    pub const HIERARCHY_REQUEST_ERR: u16 = Code::HierarchyRequestErr.code();

    /// Legacy error code for WrongDocumentError (value: 4).
    #[boa(constant)]
    pub const WRONG_DOCUMENT_ERR: u16 = Code::WrongDocumentErr.code();

    /// Legacy error code for InvalidCharacterError (value: 5).
    #[boa(constant)]
    pub const INVALID_CHARACTER_ERR: u16 = Code::InvalidCharacterErr.code();

    /// Legacy error code for NoModificationAllowedError (value: 7).
    #[boa(constant)]
    pub const NO_MODIFICATION_ALLOWED_ERR: u16 = Code::NoModificationAllowedErr.code();

    /// Legacy error code for NotFoundError (value: 8).
    #[boa(constant)]
    pub const NOT_FOUND_ERR: u16 = Code::NotFoundErr.code();

    /// Legacy error code for NotSupportedError (value: 9).
    #[boa(constant)]
    pub const NOT_SUPPORTED_ERR: u16 = Code::NotSupportedErr.code();

    /// Legacy error code for InUseAttributeError (value: 10).
    #[boa(constant)]
    pub const INUSE_ATTRIBUTE_ERR: u16 = Code::InuseAttributeErr.code();

    /// Legacy error code for InvalidStateError (value: 11).
    #[boa(constant)]
    pub const INVALID_STATE_ERR: u16 = Code::InvalidStateErr.code();

    /// Legacy error code for SyntaxError (value: 12).
    #[boa(constant)]
    pub const SYNTAX_ERR: u16 = Code::SyntaxErr.code();

    /// Legacy error code for InvalidModificationError (value: 13).
    #[boa(constant)]
    pub const INVALID_MODIFICATION_ERR: u16 = Code::InvalidModificationErr.code();

    /// Legacy error code for NamespaceError (value: 14).
    #[boa(constant)]
    pub const NAMESPACE_ERR: u16 = Code::NamespaceErr.code();

    /// Legacy error code for InvalidAccessError (value: 15).
    #[boa(constant)]
    pub const INVALID_ACCESS_ERR: u16 = Code::InvalidAccessErr.code();

    /// Legacy error code for TypeMismatchError (value: 17).
    #[boa(constant)]
    pub const TYPE_MISMATCH_ERR: u16 = Code::TypeMismatchErr.code();

    /// Legacy error code for SecurityError (value: 18).
    #[boa(constant)]
    pub const SECURITY_ERR: u16 = Code::SecurityErr.code();

    /// Legacy error code for NetworkError (value: 19).
    #[boa(constant)]
    pub const NETWORK_ERR: u16 = Code::NetworkErr.code();

    /// Legacy error code for AbortError (value: 20).
    #[boa(constant)]
    pub const ABORT_ERR: u16 = Code::AbortErr.code();

    /// Legacy error code for URLMismatchError (value: 21).
    #[boa(constant)]
    pub const URL_MISMATCH_ERR: u16 = Code::UrlMismatchErr.code();

    /// Legacy error code for QuotaExceededError (value: 22).
    #[boa(constant)]
    pub const QUOTA_EXCEEDED_ERR: u16 = Code::QuotaExceededErr.code();

    /// Legacy error code for TimeoutError (value: 23).
    #[boa(constant)]
    pub const TIMEOUT_ERR: u16 = Code::TimeoutErr.code();

    /// Legacy error code for InvalidNodeTypeError (value: 24).
    #[boa(constant)]
    pub const INVALID_NODE_TYPE_ERR: u16 = Code::InvalidNodeTypeErr.code();

    /// Legacy error code for DataCloneError (value: 25).
    #[boa(constant)]
    pub const DATA_CLONE_ERR: u16 = Code::DataCloneErr.code();

    /// The data constructor for DOMException.
    ///
    /// [spec]: https://webidl.spec.whatwg.org/#dom-domexception-domexception
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/DOMException/DOMException
    #[boa(constructor)]
    pub fn constructor(message: Option<JsString>, name: Option<JsString>) -> Self {
        // 1. If message is not given, let message be the empty string.
        let message = message.unwrap_or_else(|| js_string!());

        // 2. If name is not given, let name be "Error".
        let name = name.unwrap_or_else(|| js_string!("Error"));

        Self { message, name }
    }

    /// Returns the prototype to inherit from (Error.prototype).
    #[boa(inherit)]
    fn inherit(context: &mut Context) -> JsResult<JsObject> {
        Ok(context.intrinsics().constructors().error().prototype())
    }

    /// Get the message property.
    #[boa(getter)]
    pub fn message(&self) -> JsString {
        self.message.clone()
    }

    /// Get the name property.
    #[boa(getter)]
    pub fn name(&self) -> JsString {
        self.name.clone()
    }

    /// Get the code property.
    ///
    /// Returns the legacy numeric error code for this exception, or 0 if the name doesn't match
    /// any of the legacy error names.
    ///
    /// [spec]: https://webidl.spec.whatwg.org/#dom-domexception-code
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/DOMException/code
    #[boa(getter)]
    pub fn code(&self) -> u16 {
        Code::from_name(self.name.as_str())
            .map(|c| c.code())
            .unwrap_or(0)
    }
}

/// Register the `DOMException` class into the realm/context.
///
/// # Errors
/// This will error if the context or realm cannot register the class.
pub fn register(context: &mut Context) -> JsResult<()> {
    context.register_global_class::<DOMException>()
}

#[cfg(test)]
mod tests;
