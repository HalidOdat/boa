//! Boa's implementation of the WHATWG DOM `Event` interface.
//!
//! The Event interface represents an event which takes place in the DOM.
//!
//! More information:
//!  - [MDN documentation][mdn]
//!  - [WHATWG DOM specification][spec]
//!
//! [spec]: https://dom.spec.whatwg.org/#interface-event
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event

use boa_engine::{Context, Finalize, JsData, JsResult, Trace, boa_class, boa_module};

/// The [`Event`][mdn] interface represents an event which takes place in the DOM.
///
/// [spec]: https://dom.spec.whatwg.org/#interface-event
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event
#[derive(Debug, Default, Clone, JsData, Trace, Finalize)]
pub struct Event {
    // Event implementation details will go here
}

#[boa_class]
impl Event {
    /// The [`Event()`][mdn] constructor returns a new `Event` object.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-event
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/Event
    #[boa(constructor)]
    pub fn constructor() -> JsResult<Self> {
        Ok(Self::default())
    }
}

/// JavaScript module containing the Event class.
#[boa_module]
pub mod js_module {
    type Event = super::Event;

    /// The `NONE` constant (value 0) represents no event phase.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-none
    pub const NONE: u32 = 0;
}

/// Register the `Event` class into the realm/context.
///
/// # Errors
/// This will error if the context or realm cannot register the class.
pub fn register(context: &mut Context) -> JsResult<()> {
    js_module::boa_register(None, context)
}
