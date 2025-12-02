//! Boa's implementation of the WHATWG DOM `EventTarget` interface.
//!
//! The EventTarget interface represents an object that can receive events and may have listeners for them.
//!
//! More information:
//!  - [MDN documentation][mdn]
//!  - [WHATWG DOM specification][spec]
//!
//! [spec]: https://dom.spec.whatwg.org/#interface-eventtarget
//! [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/EventTarget

use boa_engine::{
    Context, Finalize, JsData, JsNativeError, JsObject, JsResult, JsString, JsValue, Trace,
    boa_class, boa_module, js_string,
};
use boa_gc::{Gc, GcRefCell};
use std::collections::HashMap;

use super::Event;

#[cfg(test)]
mod tests;

/// The [`EventTarget`][mdn] interface represents an object that can receive events and may have listeners for them.
///
/// [spec]: https://dom.spec.whatwg.org/#interface-eventtarget
/// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/EventTarget
#[derive(Debug, Clone, JsData, Trace, Finalize)]
pub struct EventTarget {
    /// Event listeners mapped by event type.
    listeners: Gc<GcRefCell<HashMap<JsString, Vec<EventListener>>>>,
}

/// An event listener.
#[derive(Debug, Clone, Trace, Finalize)]
struct EventListener {
    /// The callback function to invoke.
    callback: JsObject,
    /// Whether to capture events in the capturing phase.
    capture: bool,
    /// Whether the listener is passive.
    passive: bool,
    /// Whether the listener should be invoked at most once.
    once: bool,
}

#[boa_class]
impl EventTarget {
    /// The [`EventTarget()`][mdn] constructor returns a new `EventTarget` object.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-eventtarget-eventtarget
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/EventTarget
    #[boa(constructor)]
    pub fn constructor(_context: &mut Context) -> JsResult<Self> {
        Ok(Self {
            listeners: Gc::new(GcRefCell::new(HashMap::new())),
        })
    }

    /// The [`addEventListener()`][mdn] method adds a new event listener.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-eventtarget-addeventlistener
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/addEventListener
    pub fn add_event_listener(
        &mut self,
        event_type: JsString,
        callback: JsValue,
        options: JsValue,
        context: &mut Context,
    ) -> JsResult<JsValue> {
        // https://dom.spec.whatwg.org/#dom-eventtarget-addeventlistener
        //
        // 1. Let capture, passive, once, and signal be the result of flattening options.
        // TODO: signal/AbortSignal not implemented yet

        let (capture, passive, once) = if let Some(options_obj) = options.as_object() {
            let capture = options_obj
                .get(js_string!("capture"), context)?
                .to_boolean();
            let passive = options_obj
                .get(js_string!("passive"), context)?
                .to_boolean();
            let once = options_obj.get(js_string!("once"), context)?.to_boolean();
            (capture, passive, once)
        } else if options.is_boolean() {
            // Legacy boolean parameter for capture
            (options.to_boolean(), false, false)
        } else {
            (false, false, false)
        };

        // 2. If callback is null, then return.
        let callback = if let Some(obj) = callback.as_object() {
            obj.clone()
        } else {
            return Ok(JsValue::undefined());
        };

        // 3. If signal is not null, add the following abort steps to signal:
        //    3.1. Remove an event listener with this, type, callback, and capture.
        // TODO: AbortSignal not implemented

        // 4. If this's event listener list does not contain an event listener whose type is type,
        //    callback is callback, and capture is capture, then append a new event listener to this's
        //    event listener list whose type is type, callback is callback, capture is capture, passive is
        //    passive, once is once, signal is signal, and removed is false.
        let listener = EventListener {
            callback,
            capture,
            passive,
            once,
        };

        let mut listeners_map = self.listeners.borrow_mut();
        let listeners = listeners_map.entry(event_type).or_insert_with(Vec::new);

        let exists = listeners.iter().any(|l| {
            JsValue::same_value(
                &JsValue::from(l.callback.clone()),
                &JsValue::from(listener.callback.clone()),
            ) && l.capture == listener.capture
        });

        if !exists {
            listeners.push(listener);
        }

        Ok(JsValue::undefined())
    }

    /// The [`removeEventListener()`][mdn] method removes an event listener.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-eventtarget-removeeventlistener
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/removeEventListener
    pub fn remove_event_listener(
        &mut self,
        event_type: JsString,
        callback: JsValue,
        options: JsValue,
        context: &mut Context,
    ) -> JsResult<JsValue> {
        // https://dom.spec.whatwg.org/#dom-eventtarget-removeeventlistener
        //
        // 1. Let capture be the result of flattening options.
        let capture = if let Some(options_obj) = options.as_object() {
            options_obj
                .get(js_string!("capture"), context)?
                .to_boolean()
        } else if options.is_boolean() {
            options.to_boolean()
        } else {
            false
        };

        // 2. If callback is null, then return.
        let callback = if let Some(obj) = callback.as_object() {
            obj.clone()
        } else {
            return Ok(JsValue::undefined());
        };

        // 3. If there is an event listener in this's event listener list whose type is type,
        //    callback is callback, and capture is capture, then remove that event listener from
        //    this's event listener list.
        let mut listeners_map = self.listeners.borrow_mut();
        if let Some(listeners) = listeners_map.get_mut(&event_type) {
            listeners.retain(|l| {
                !(JsValue::same_value(
                    &JsValue::from(l.callback.clone()),
                    &JsValue::from(callback.clone()),
                ) && l.capture == capture)
            });

            if listeners.is_empty() {
                listeners_map.remove(&event_type);
            }
        }

        Ok(JsValue::undefined())
    }

    /// The [`dispatchEvent()`][mdn] method dispatches an event to this `EventTarget`.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#dom-eventtarget-dispatchevent
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/EventTarget/dispatchEvent
    pub fn dispatch_event(&mut self, event: JsValue, context: &mut Context) -> JsResult<JsValue> {
        // https://dom.spec.whatwg.org/#dom-eventtarget-dispatchevent
        //
        // 1. If event's dispatch flag is set, or if its initialized flag is not set, then throw
        //    an "InvalidStateError" DOMException.
        let event_obj = event
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("Event must be an object"))?;

        let (dispatch_flag, initialized_flag) = {
            let event_data = event_obj
                .downcast_ref::<Event>()
                .ok_or_else(|| JsNativeError::typ().with_message("Not a valid Event object"))?;
            (event_data.dispatch_flag, event_data.initialized_flag)
        };

        if dispatch_flag {
            return Err(JsNativeError::error()
                .with_message("InvalidStateError: Event is already being dispatched")
                .into());
        }

        if !initialized_flag {
            return Err(JsNativeError::error()
                .with_message("InvalidStateError: Event is not initialized")
                .into());
        }

        // 2. Initialize event's isTrusted attribute to false.
        {
            let mut event_data = event_obj
                .downcast_mut::<Event>()
                .ok_or_else(|| JsNativeError::typ().with_message("Not a valid Event object"))?;
            event_data.is_trusted = false;
        }

        // 3. Return the result of dispatching event to this.
        self.dispatch(event_obj.clone(), context)
    }
}

impl EventTarget {
    /// Dispatch an event to this EventTarget.
    ///
    /// This implements the "dispatch" algorithm from the spec.
    ///
    /// [spec]: https://dom.spec.whatwg.org/#concept-event-dispatch
    fn dispatch(&mut self, event: JsObject, context: &mut Context) -> JsResult<JsValue> {
        // 1. Set event's dispatch flag.
        let event_type = {
            let mut event_data = event
                .downcast_mut::<Event>()
                .ok_or_else(|| JsNativeError::typ().with_message("Not a valid Event object"))?;
            event_data.dispatch_flag = true;
            event_data.event_type.clone()
        };

        // 2. Let targetOverride be target, if legacy target override flag is not given, and target's associated Document otherwise.
        // TODO: Legacy target override flag not implemented (requires Document)

        // 3. Let activationTarget be null.
        // TODO: Activation behavior not implemented

        // 4. Let relatedTarget be the result of retargeting event's relatedTarget against target.
        // TODO: relatedTarget and retargeting not implemented (requires shadow DOM)

        // 5. If target is not relatedTarget or target is event's relatedTarget, then:
        // TODO: For now we always proceed since relatedTarget is not implemented

        // 5.1. Let touchTargets be a new list.
        // TODO: Touch targets not implemented (requires touch events)

        // 5.2. For each touchTarget of event's touch target list, append the result of
        //      retargeting touchTarget against target to touchTargets.
        // TODO: Touch targets not implemented

        // 5.3. Append to an event path with event, target, targetOverride, relatedTarget,
        //      touchTargets, and false.
        // TODO: Event path not implemented (requires shadow DOM for full propagation)
        //       For now, we simulate a simple path with just the target.

        // 5.4. Let isActivationEvent be true, if event is a MouseEvent object and event's
        //      type attribute is "click"; otherwise false.
        // TODO: Activation events not implemented

        // 5.5. If isActivationEvent is true and target has activation behavior, then set activationTarget to target.
        // TODO: Activation behavior not implemented

        // 5.6. Let slottable be target, if target is a slottable and is assigned, and null otherwise.
        // TODO: Slottables not implemented (requires shadow DOM)

        // 5.7. Let slot-in-closed-tree be false.
        // TODO: Shadow DOM not implemented

        // 5.8. Let parent be the result of invoking target's get the parent with event.
        // TODO: Parent navigation not implemented (requires node tree structure)
        //       For now, parent is always None since we have flat EventTargets

        // 5.9. While parent is non-null:
        // TODO: Propagation through parent chain not implemented
        //       This would require a tree structure of EventTargets

        // 6. Let clearTargetsStruct be the last struct in event's path whose shadow-adjusted
        //    target is non-null.
        // TODO: Shadow DOM not implemented

        // 7. Let clearTargets be true if clearTargetsStruct's shadow-adjusted target,
        //    clearTargetsStruct's relatedTarget, or an EventTarget object in
        //    clearTargetsStruct's touch target list is a node and its root is a shadow root; otherwise false.
        // TODO: Shadow DOM not implemented

        // 8. If activationTarget is non-null and activationTarget has legacy-pre-activation behavior, then run activationTarget's legacy-pre-activation behavior.
        // TODO: Activation behavior not implemented

        // 9. For each struct in event's path, in reverse order:
        // TODO: Full event path with capturing phase not implemented
        //       For now, we only handle the target phase

        // 10. For each struct in event's path:
        // This is the bubbling phase iteration
        // TODO: Full event path with bubbling phase not implemented
        //       For now, we simulate just the AT_TARGET phase

        // Set event phase to AT_TARGET and set target
        // TODO: Should set event.target to the actual EventTarget object, but we don't have
        //       a way to get a JsObject reference to `self` here
        {
            let mut event_data = event
                .downcast_mut::<Event>()
                .ok_or_else(|| JsNativeError::typ().with_message("Not a valid Event object"))?;
            event_data.event_phase = super::EventPhase::AtTarget;
            // event_data.target = Some(target_object); // TODO: Cannot get JsObject for self
        }

        // Invoke listeners at target (simplified - no capturing/bubbling)
        let listeners_map = self.listeners.borrow();
        let listeners = listeners_map.get(&event_type).cloned();
        drop(listeners_map);

        if let Some(listeners) = listeners {
            for listener in &listeners {
                // Set passive listener flag if needed
                if listener.passive {
                    if let Some(mut event_data) = event.downcast_mut::<Event>() {
                        event_data.in_passive_listener_flag = true;
                    }
                }

                // Call the listener callback
                // The spec says to call with event as argument and handle exceptions
                listener
                    .callback
                    .call(
                        &JsValue::undefined(),
                        &[JsValue::from(event.clone())],
                        context,
                    )
                    .ok(); // Ignore exceptions per spec

                // Unset passive listener flag
                if listener.passive {
                    if let Some(mut event_data) = event.downcast_mut::<Event>() {
                        event_data.in_passive_listener_flag = false;
                    }
                }

                // Remove listener if once flag is set
                if listener.once {
                    let mut listeners_map = self.listeners.borrow_mut();
                    if let Some(listeners_vec) = listeners_map.get_mut(&event_type) {
                        listeners_vec.retain(|l| {
                            !(JsValue::same_value(
                                &JsValue::from(l.callback.clone()),
                                &JsValue::from(listener.callback.clone()),
                            ) && l.capture == listener.capture)
                        });
                    }
                }

                // Check if immediate propagation was stopped
                if let Some(event_data) = event.downcast_ref::<Event>() {
                    if event_data.stop_immediate_propagation_flag {
                        break;
                    }
                }
            }
        }

        // 11. Set event's eventPhase attribute to NONE.
        {
            let mut event_data = event
                .downcast_mut::<Event>()
                .ok_or_else(|| JsNativeError::typ().with_message("Not a valid Event object"))?;
            event_data.event_phase = super::EventPhase::None;
        }

        // 12. Set event's currentTarget attribute to null.
        // TODO: currentTarget not implemented yet

        // 13. Set event's path to the empty list.
        // TODO: Event path not implemented

        // 14. Unset event's dispatch flag, stop propagation flag, and stop immediate propagation flag.
        {
            let mut event_data = event
                .downcast_mut::<Event>()
                .ok_or_else(|| JsNativeError::typ().with_message("Not a valid Event object"))?;
            event_data.dispatch_flag = false;
            event_data.stop_propagation_flag = false;
            event_data.stop_immediate_propagation_flag = false;
        }

        // 15. If clearTargets, then:
        // TODO: Shadow DOM not implemented

        // 16. If activationTarget is non-null, then:
        // TODO: Activation behavior not implemented

        // 17. Return false if event's canceled flag is set; otherwise true.
        let cancelled_flag = {
            let event_data = event
                .downcast_ref::<Event>()
                .ok_or_else(|| JsNativeError::typ().with_message("Event object lost"))?;
            event_data.cancelled_flag
        };

        Ok(JsValue::from(!cancelled_flag))
    }
}

/// JavaScript module containing the EventTarget class.
#[boa_module]
pub mod js_module {
    type EventTarget = super::EventTarget;
}

/// Registers the `EventTarget` class in the given context.
///
/// # Errors
/// This will error if the context or realm cannot register the class.
pub fn register(context: &mut Context) -> JsResult<()> {
    js_module::boa_register(None, context)
}
