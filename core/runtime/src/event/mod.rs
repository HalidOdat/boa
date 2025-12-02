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

pub mod target;
pub use target::EventTarget;

#[cfg(test)]
mod tests;

use boa_engine::{
    Context, Finalize, JsData, JsObject, JsResult, JsString, JsValue, Trace, boa_class, boa_module,
    context::time::JsInstant, value::Convert, value::TryFromJs,
};

/// The phase of the event flow.
///
/// [spec]: https://dom.spec.whatwg.org/#dom-event-eventphase
#[repr(u16)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Trace, Finalize)]
/// SAFETY: `EventPhase` is a simple enum with no fields and contains no `Gc` pointers.
#[boa_gc(unsafe_empty_trace)]
pub enum EventPhase {
    /// No event phase (value 0).
    #[default]
    None = 0,
    /// Capturing phase (value 1).
    Capturing = 1,
    /// At target phase (value 2).
    AtTarget = 2,
    /// Bubbling phase (value 3).
    Bubbling = 3,
}

impl EventPhase {
    /// Returns the numeric value of the event phase.
    pub const fn as_u16(self) -> u16 {
        self as u16
    }
}

/// Dictionary for Event initialization.
///
/// [spec]: https://dom.spec.whatwg.org/#dictdef-eventinit
#[derive(Debug, Default, Clone, TryFromJs, Trace, Finalize)]
pub struct EventInit {
    /// Whether the event bubbles.
    bubbles: Option<bool>,
    /// Whether the event is cancelable.
    cancelable: Option<bool>,
    /// Whether the event is composed.
    composed: Option<bool>,
}

/// The [`Event`][mdn] interface represents an event which takes place in the DOM.
///
/// [spec]: https://dom.spec.whatwg.org/#interface-event
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event
#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct Event {
    /// The type of event.
    event_type: JsString,
    /// Whether the event bubbles.
    bubbles: bool,
    /// Whether the event is cancelable.
    cancelable: bool,
    /// Whether the event is composed.
    composed: bool,
    /// The current event phase.
    event_phase: EventPhase,
    /// The time at which the event was created.
    /// SAFETY: `JsInstant` does not contain any `Gc` pointers and is safe to ignore during tracing.
    #[unsafe_ignore_trace]
    time_stamp: JsInstant,
    /// Whether the event's default action has been prevented.
    cancelled_flag: bool,
    /// Whether the event was dispatched by the user agent.
    is_trusted: bool,
    /// Whether the event's propagation has been stopped.
    stop_propagation_flag: bool,
    /// Whether the event's propagation has been stopped immediately.
    stop_immediate_propagation_flag: bool,
    /// Whether the event is currently being dispatched.
    dispatch_flag: bool,
    /// Whether the event is being handled by a passive listener.
    in_passive_listener_flag: bool,
    /// Whether the event has been initialized.
    initialized_flag: bool,
    /// The event's target.
    target: Option<JsObject>,
}

#[boa_class]
impl Event {
    /// The [`Event()`][mdn] constructor returns a new `Event` object.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-event
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/Event
    #[boa(constructor)]
    pub fn constructor(
        Convert(ref event_type): Convert<JsString>,
        event_init: Option<EventInit>,
        context: &mut Context,
    ) -> JsResult<Self> {
        let event_init = event_init.unwrap_or_default();

        Ok(Self {
            event_type: event_type.clone(),
            bubbles: event_init.bubbles.unwrap_or(false),
            cancelable: event_init.cancelable.unwrap_or(false),
            composed: event_init.composed.unwrap_or(false),
            event_phase: EventPhase::None,
            time_stamp: context.clock().now(),
            cancelled_flag: false,
            is_trusted: false,
            stop_propagation_flag: false,
            stop_immediate_propagation_flag: false,
            dispatch_flag: false,
            in_passive_listener_flag: false,
            initialized_flag: true,
            target: None,
        })
    }

    /// The [`type`][mdn] read-only property returns the type of the event.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-type
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/type
    #[boa(getter)]
    #[boa(rename = "type")]
    pub fn r#type(&self) -> JsString {
        self.event_type.clone()
    }

    /// The [`bubbles`][mdn] read-only property indicates whether the event bubbles.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-bubbles
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/bubbles
    #[boa(getter)]
    pub fn bubbles(&self) -> bool {
        self.bubbles
    }

    /// The [`cancelable`][mdn] read-only property indicates whether the event is cancelable.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-cancelable
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/cancelable
    #[boa(getter)]
    pub fn cancelable(&self) -> bool {
        self.cancelable
    }

    /// The [`composed`][mdn] read-only property indicates whether the event will propagate across the shadow DOM boundary.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-composed
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/composed
    #[boa(getter)]
    pub fn composed(&self) -> bool {
        self.composed
    }

    /// The [`eventPhase`][mdn] read-only property indicates which phase of the event flow is currently being evaluated.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-eventphase
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/eventPhase
    #[boa(getter)]
    pub fn event_phase(&self) -> u16 {
        self.event_phase.as_u16()
    }

    /// The [`timeStamp`][mdn] read-only property returns the time (in milliseconds) at which the event was created.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-timestamp
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/timeStamp
    #[boa(getter)]
    pub fn time_stamp(&self, context: &mut Context) -> f64 {
        (context.clock().now() - self.time_stamp).as_millis() as f64
    }

    /// The [`defaultPrevented`][mdn] read-only property indicates whether the call to `preventDefault()` canceled the event.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-defaultprevented
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/defaultPrevented
    #[boa(getter)]
    pub fn default_prevented(&self) -> bool {
        self.cancelled_flag
    }

    /// The [`isTrusted`][mdn] read-only property indicates whether the event was dispatched by the user agent.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-istrusted
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/isTrusted
    #[boa(getter)]
    pub fn is_trusted(&self) -> bool {
        self.is_trusted
    }

    /// The [`target`][mdn] read-only property returns the object to which the event was dispatched.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-target
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/target
    #[boa(getter)]
    pub fn target(&self) -> JsValue {
        self.target
            .as_ref()
            .map(|t| t.clone().into())
            .unwrap_or(JsValue::null())
    }

    /// The [`stopPropagation()`][mdn] method prevents further propagation of the current event.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-stoppropagation
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/stopPropagation
    pub fn stop_propagation(&mut self) {
        self.stop_propagation_flag = true;
    }

    /// The [`preventDefault()`][mdn] method tells the user agent that if the event does not get explicitly handled,
    /// its default action should not be taken as it normally would be.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-preventdefault
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/preventDefault
    pub fn prevent_default(&mut self) {
        // 1. If this's cancelable attribute value is true and this's in passive listener flag is unset, then set this's canceled flag.
        if self.cancelable && !self.in_passive_listener_flag {
            self.cancelled_flag = true;
        }
    }

    /// The [`stopImmediatePropagation()`][mdn] method prevents other listeners of the same event from being called.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-stopimmediatepropagation
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/stopImmediatePropagation
    pub fn stop_immediate_propagation(&mut self) {
        self.stop_propagation_flag = true;
        self.stop_immediate_propagation_flag = true;
    }

    /// The [`initEvent()`][mdn] method initializes the value of an event created using `Document.createEvent()`.
    ///
    /// Note: This method is deprecated and should not be used for new code.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-initevent
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Event/initEvent
    #[allow(deprecated)]
    pub fn init_event(
        &mut self,
        Convert(ref event_type): Convert<JsString>,
        bubbles: Option<bool>,
        cancelable: Option<bool>,
    ) {
        // 1. If this's dispatch flag is set, then return.
        if self.dispatch_flag {
            return;
        }

        // 2. Initialize this with type, bubbles, and cancelable.
        self.initialize(
            event_type.clone(),
            bubbles.unwrap_or(false),
            cancelable.unwrap_or(false),
        );
    }

    /// The `NONE` constant (value 0) represents no event phase.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-none
    #[boa(constant)]
    pub const NONE: u32 = 0;

    /// The `CAPTURING_PHASE` constant (value 1) represents the capturing phase.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-capturing_phase
    #[boa(constant)]
    pub const CAPTURING_PHASE: u32 = 1;

    /// The `AT_TARGET` constant (value 2) represents the target phase.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-at_target
    #[boa(constant)]
    pub const AT_TARGET: u32 = 2;

    /// The `BUBBLING_PHASE` constant (value 3) represents the bubbling phase.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-event-bubbling_phase
    #[boa(constant)]
    pub const BUBBLING_PHASE: u32 = 3;
}

impl Event {
    /// Initialize an event.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#concept-event-initialize
    fn initialize(&mut self, event_type: JsString, bubbles: bool, cancelable: bool) {
        // 1. Set event's initialized flag.
        self.initialized_flag = true;

        // 2. Unset event's stop propagation flag, stop immediate propagation flag and canceled flag.
        self.stop_propagation_flag = false;
        self.stop_immediate_propagation_flag = false;
        self.cancelled_flag = false;

        // 3. Set event's isTrusted attribute to false.
        self.is_trusted = false;

        // 4. Set event's target to null.
        self.target = None;

        // 5. Set event's type attribute to type.
        self.event_type = event_type;

        // 6. Set event's bubbles attribute to bubbles.
        self.bubbles = bubbles;

        // 7. Set event's cancelable attribute to cancelable.
        self.cancelable = cancelable;
    }
}

/// JavaScript module containing the Event class.
#[boa_module]
pub mod js_module {
    type Event = super::Event;
}

/// Register the `Event` class into the realm/context.
///
/// # Errors
/// This will error if the context or realm cannot register the class.
pub fn register(context: &mut Context) -> JsResult<()> {
    js_module::boa_register(None, context)
}
