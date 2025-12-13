//! Module containing the `Response` JavaScript class and its helpers, implemented as
//! [`JsResponse`].
//!
//! See the [Response interface documentation][mdn] for more information.
//!
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Response

use crate::fetch::headers::JsHeaders;
use boa_engine::object::builtins::{JsPromise, JsUint8Array};
use boa_engine::value::{Convert, TryFromJs, TryIntoJs};
use boa_engine::{
    Context, JsData, JsNativeError, JsResult, JsString, JsValue, boa_class, js_error, js_str,
    js_string,
};
use boa_gc::{Finalize, Trace};
use http::StatusCode;
use std::rc::Rc;

/// The type read-only property of the Response interface contains the type of the
/// response. The type determines whether scripts are able to access the response body
/// and headers.
///
/// See <https://developer.mozilla.org/en-US/docs/Web/API/Response/type>.
#[derive(Debug, Copy, Clone)]
pub enum ResponseType {
    /// This applies in any of the following cases:
    ///
    /// The request is same-origin.
    /// The requested URL's scheme is `data:`.
    /// The request's mode is `navigate` or `websocket`.
    ///
    /// With this type, all response headers are exposed except Set-Cookie.
    Basic,

    /// The request was cross-origin and was successfully processed using CORS. With this
    /// type, only CORS-safelisted response headers are exposed.
    Cors,

    /// A network error occurred. The status property is set to 0, `body` is null, headers
    /// are empty and immutable.
    Error,

    /// A response to a cross-origin request whose mode was set to no-cors. The status
    /// property is set to 0, `body` is null, headers are empty and immutable.
    Opaque,

    /// A response to a request whose redirect option was set to manual and which was
    /// redirected by the server. The status property is set to 0, `body` is null, headers
    /// are empty and immutable.
    OpaqueRedirect,
}

impl ResponseType {
    /// Return the JavaScript String representing this response type.
    #[must_use]
    pub fn to_string(self) -> JsString {
        match self {
            ResponseType::Basic => js_string!("basic"),
            ResponseType::Cors => js_string!("cors"),
            ResponseType::Error => js_string!("error"),
            ResponseType::Opaque => js_string!("opaque"),
            ResponseType::OpaqueRedirect => js_string!("opaqueredirect"),
        }
    }
}

impl TryFromJs for ResponseType {
    fn try_from_js(value: &JsValue, context: &mut Context) -> JsResult<Self> {
        let value_str = value.to_string(context)?;
        if value_str == js_str!("basic") {
            Ok(ResponseType::Basic)
        } else if value_str == js_str!("cors") {
            Ok(ResponseType::Cors)
        } else if value_str == js_str!("error") {
            Ok(ResponseType::Error)
        } else if value_str == js_str!("opaque") {
            Ok(ResponseType::Opaque)
        } else if value_str == js_str!("opaqueredirect") {
            Ok(ResponseType::OpaqueRedirect)
        } else {
            Err(js_error!(TypeError: "Invalid response type value"))
        }
    }
}

impl TryIntoJs for ResponseType {
    fn try_into_js(&self, _: &mut Context) -> JsResult<JsValue> {
        Ok(self.to_string().into())
    }
}

/// The `Response` interface of the Fetch API represents the response to a request.
//
// You can create a new Response object using the `Response` constructor, but you
// are more likely to encounter a `Response` object being returned as the result of
// another API operation.
#[derive(Clone, Debug, Trace, Finalize, JsData)]
pub struct JsResponse {
    url: JsString,

    #[unsafe_ignore_trace]
    r#type: ResponseType,

    #[unsafe_ignore_trace]
    status: Option<StatusCode>,

    status_text: JsString,

    headers: JsHeaders,

    #[unsafe_ignore_trace]
    body: Option<Rc<Vec<u8>>>,
}

impl JsResponse {
    /// Create a new instance from the HTTP response and the URL that requested it.
    #[must_use]
    pub fn basic(url: JsString, inner: http::Response<Vec<u8>>) -> Self {
        let (parts, body) = inner.into_parts();
        let status = Some(parts.status);
        let status_text = JsString::from(status.and_then(|s| s.canonical_reason()).unwrap_or(""));
        let headers = JsHeaders::from_http(parts.headers);
        let body = if body.is_empty() {
            None
        } else {
            Some(Rc::new(body))
        };

        Self {
            url,
            r#type: ResponseType::Basic,
            status,
            status_text,
            headers,
            body,
        }
    }

    /// Create a new instance of [`JsResponse`] that is an error.
    #[must_use]
    pub fn error() -> Self {
        Self {
            url: js_string!(""),
            r#type: ResponseType::Error,
            status: None,
            status_text: JsString::default(),
            headers: JsHeaders::default(),
            body: None,
        }
    }

    /// Return a copy of the body.
    #[must_use]
    pub fn body(&self) -> Option<Rc<Vec<u8>>> {
        self.body.clone()
    }

    /// Initialize a response.
    ///
    /// This implements the "initialize a response" algorithm from the Fetch specification:
    /// https://fetch.spec.whatwg.org/#initialize-a-response
    ///
    /// To initialize a response, given a Response object response, ResponseInit init,
    /// and null or a body with type body:
    ///
    /// # Errors
    /// Returns an error if the response cannot be initialized.
    fn initialize(
        options: JsResponseOptions,
        body_with_type: Option<(Vec<u8>, Option<JsString>)>,
        _context: &mut Context,
    ) -> JsResult<JsResponse> {
        // 1. If init["status"] is not in the range 200 to 599, inclusive, then throw a RangeError.
        let status = options.status.unwrap_or(200);
        if !(200..=599).contains(&status) {
            return Err(js_error!(RangeError: "Response status must be between 200 and 599"));
        }

        // 2. If init["statusText"] is not the empty string and does not match the reason-phrase
        //    token production, then throw a TypeError.
        if let Some(ref status_text_value) = options.status_text {
            // Validate status text contains only allowed characters (ASCII printable except CTL)
            let status_text_std = status_text_value.to_std_string_escaped();
            for ch in status_text_std.chars() {
                if ch < ' ' || ch > '~' || ch == '\x7F' {
                    return Err(
                        js_error!(TypeError: "Response statusText contains invalid characters"),
                    );
                }
            }
        }

        // 3. Set response's response's status to init["status"].
        let status = StatusCode::from_u16(status)
            .map_err(|_| js_error!(RangeError: "Invalid status code"))?;

        // 4. Set response's response's status message to init["statusText"].
        let status_text = options.status_text.clone().unwrap_or_else(|| {
            status
                .canonical_reason()
                .map_or_else(JsString::default, JsString::from)
        });

        // 5. If init["headers"] exists, then fill response's headers with init["headers"].
        let mut headers = options.headers.clone().unwrap_or_default();

        // 6. If body is non-null, then:
        let body = if let Some((body_data, body_type)) = body_with_type {
            // 1. If response's status is a null body status, then throw a TypeError.
            //
            //      Note: 101 and 103 are included in null body status due to their use elsewhere.
            //      They do not affect this step.
            if matches!(status.as_u16(), 101 | 103 | 204 | 205 | 304) {
                return Err(
                    js_error!(TypeError: "Response with null body status cannot have a body"),
                );
            }

            // 2. Set response's body to body's body.
            let body = Some(Rc::new(body_data));

            // 3. If body's type is non-null and response's header list does not contain
            //      `Content-Type`, then append (`Content-Type`, body's type) to response's header list.
            if let Some(content_type) = body_type {
                let has_content_type = headers.has(Convert::from("content-type".to_string()))?;
                if !has_content_type {
                    headers.append(
                        Convert::from("content-type".to_string()),
                        Convert::from(content_type.to_std_string_escaped()),
                    )?;
                }
            }

            body
        } else {
            None
        };

        Ok(JsResponse {
            url: js_string!(""),
            r#type: ResponseType::Basic,
            status: Some(status),
            status_text,
            headers,
            body,
        })
    }
}

/// Options used in the construction of a `Response` object.
#[derive(Debug, Clone, Default, TryFromJs, TryIntoJs, Trace, Finalize, JsData)]
#[boa(rename_all = "camelCase")]
pub struct JsResponseOptions {
    status: Option<u16>,
    status_text: Option<JsString>,
    headers: Option<JsHeaders>,
}

#[boa_class(rename = "Response")]
#[boa(rename_all = "camelCase")]
impl JsResponse {
    #[boa(static)]
    #[boa(rename = "error")]
    fn error_() -> Self {
        Self::error()
    }

    /// [`new Response(body, init)`][spec]
    ///
    /// [spec]: https://fetch.spec.whatwg.org/#dom-response
    #[boa(constructor)]
    fn constructor(
        body: Option<JsValue>,
        options: Option<JsResponseOptions>,
        context: &mut Context,
    ) -> JsResult<Self> {
        // 1. Set this's response to a new response.
        // NOTE: Handled in initialize

        // 2. Set this's headers to a new Headers object with this's relevant realm,
        //    whose header list is this's response's header list and guard is "response".
        // NOTE: Handled in initialize

        // 3. Let bodyWithType be null.
        // 4. If body is non-null, then set bodyWithType to the result of extracting body.
        let body_with_type = if let Some(body_value) = body.filter(|v| !v.is_null_or_undefined()) {
            Some(extract_body(body_value, context)?)
        } else {
            None
        };

        // 5. Perform initialize a response given this, init, and bodyWithType.
        Self::initialize(options.unwrap_or_default(), body_with_type, context)
    }

    #[boa(getter)]
    fn status(&self) -> u16 {
        // 0 is a special case for error responses.
        self.status.map_or(0, |s| s.as_u16())
    }

    #[boa(getter)]
    fn status_text(&self) -> JsString {
        self.status_text.clone()
    }

    #[boa(getter)]
    fn headers(&self) -> JsHeaders {
        self.headers.clone()
    }

    #[boa(getter)]
    #[boa(rename = "type")]
    fn r#type(&self) -> JsString {
        self.r#type.to_string()
    }

    #[boa(getter)]
    fn url(&self) -> JsString {
        self.url.clone()
    }

    fn bytes(&self, context: &mut Context) -> JsPromise {
        let body = self.body.clone().unwrap_or_else(|| Rc::new(Vec::new()));
        JsPromise::from_async_fn(
            async move |context| {
                JsUint8Array::from_iter(body.iter().copied(), &mut context.borrow_mut())
                    .map(Into::into)
            },
            context,
        )
    }

    fn text(&self, context: &mut Context) -> JsPromise {
        let body = self.body.clone().unwrap_or_else(|| Rc::new(Vec::new()));
        JsPromise::from_async_fn(
            async move |_| {
                let body = String::from_utf8_lossy(body.as_ref());
                Ok(JsString::from(body).into())
            },
            context,
        )
    }

    fn json(&self, context: &mut Context) -> JsPromise {
        let body = self.body.clone().unwrap_or_else(|| Rc::new(Vec::new()));
        JsPromise::from_async_fn(
            async move |context| {
                let json_string = String::from_utf8_lossy(body.as_ref());
                let json = serde_json::from_str::<serde_json::Value>(&json_string)
                    .map_err(|e| JsNativeError::syntax().with_message(e.to_string()))?;

                JsValue::from_json(&json, &mut context.borrow_mut())
            },
            context,
        )
    }
}

/// Extract body bytes from a JsValue.
///
/// This implements the "extract" operation from the Fetch specification:
/// https://fetch.spec.whatwg.org/#concept-bodyinit-extract
///
/// To extract a body with type from a byte sequence or BodyInit object, with an
/// optional boolean keepalive (default false), run these steps:
///
/// # Errors
///
/// Returns an error if the body cannot be converted to bytes.
///
/// # Returns
///
/// Returns a tuple of (body bytes, optional Content-Type string).
fn extract_body(body: JsValue, context: &mut Context) -> JsResult<(Vec<u8>, Option<JsString>)> {
    // 1. Let stream be null.
    // TODO: Implement ReadableStream support

    // 2. If object is a ReadableStream object, then set stream to object.
    // TODO: Implement ReadableStream detection and handling

    // 3. Otherwise, if object is a Blob object, set stream to the result of running object's get stream.
    // TODO: Implement Blob support

    // 4. Otherwise, set stream to a new ReadableStream object, and set up stream with byte reading support.
    // TODO: Implement ReadableStream creation

    // 5. Assert: stream is a ReadableStream object.
    // TODO: Implement ReadableStream creation

    // 6. Let action be null.
    // 7. Let source be null.
    // 8. Let length be null.
    // 9. Let type be null.

    // 10. Switch on object:

    // Blob:
    // - Set source to object.
    // - Set length to object's size.
    // - If object's type attribute is not the empty byte sequence, set type to its value.
    // TODO: Implement Blob support

    // byte sequence:
    // - Set source to object.
    // NOTE: Handled by BufferSource below

    // BufferSource:
    // - Set source to a copy of the bytes held by object.
    if let Some(obj) = body.as_object() {
        if let Ok(array) = JsUint8Array::from_object(obj.clone()) {
            let data: Vec<u8> = array.iter(context).collect();
            return Ok((data, None));
        }
        // TODO: Support other TypedArray types (Int8Array, Uint16Array, etc.)
        // TODO: Support ArrayBuffer and DataView
    }

    // FormData:
    // - Set action to this step: run the multipart/form-data encoding algorithm, with object's entry list and UTF-8.
    // - Set source to object.
    // - Set length to unclear, see html/6424 for improving this.
    // - Set type to `multipart/form-data; boundary=`, followed by the multipart/form-data boundary string
    //   generated by the multipart/form-data encoding algorithm.
    // TODO: Implement FormData support

    // URLSearchParams:
    // - Set source to the result of running the application/x-www-form-urlencoded serializer with object's list.
    // - Set type to `application/x-www-form-urlencoded;charset=UTF-8`.
    // TODO: Implement URLSearchParams support

    // scalar value string:
    // - Set source to the UTF-8 encoding of object.
    // - Set type to `text/plain;charset=UTF-8`.
    if body.is_string() {
        let string = body.to_string(context)?;
        return Ok((
            string.to_std_string_escaped().into_bytes(),
            Some(js_string!("text/plain;charset=UTF-8")),
        ));
    }

    // ReadableStream:
    // - If keepalive is true, then throw a TypeError.
    // - If object is disturbed or locked, then throw a TypeError.
    // TODO: Implement ReadableStream support

    // 11. If source is a byte sequence, then set action to a step that returns source
    // and length to source's length.
    // NOTE: Handled implicitly by returning Vec<u8>

    // 12. If action is non-null, then run these steps in parallel:
    // - Run action.
    // - Whenever one or more bytes are available and stream is not errored, enqueue the result of
    //   creating a Uint8Array from the available bytes into stream.
    // - When running action is done, close stream.
    // TODO: Implement async stream processing

    // 13. Let body be a body whose stream is stream, source is source, and length is length.
    // 14. Return (body, type).
    // NOTE: In this simplified implementation, we return (bytes, type).

    // NOTE: Fallback - if it's an object, try to convert to string.
    //       This is not part of spec but needed for JavaScript compatibility.
    if body.is_object() {
        let string = body.to_string(context)?;
        return Ok((string.to_std_string_escaped().into_bytes(), None));
    }

    // NOTE: Final fallback - convert to string
    let string = body.to_string(context)?;
    Ok((string.to_std_string_escaped().into_bytes(), None))
}
