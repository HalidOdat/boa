//! Boa's implementation of ECMAScript's `IteratorRecord` and iterator prototype objects.

use crate::{
    Context, JsArgs, JsData, JsResult, JsString, JsValue,
    builtins::{Array, BuiltInBuilder, BuiltInConstructor, BuiltInObject, IntrinsicObject},
    context::intrinsics::{Intrinsics, StandardConstructor, StandardConstructors},
    error::JsNativeError,
    js_string,
    object::{JsObject, internal_methods::get_prototype_from_constructor},
    property::{Attribute, PropertyKey},
    realm::Realm,
    string::StaticJsStrings,
    symbol::JsSymbol,
    value::IntegerOrInfinity,
};
use boa_gc::{Finalize, Trace};

mod async_from_sync_iterator;
pub(crate) use async_from_sync_iterator::AsyncFromSyncIterator;

/// `IfAbruptCloseIterator ( value, iteratorRecord )`
///
/// `IfAbruptCloseIterator` is a shorthand for a sequence of algorithm steps that use an `Iterator`
/// Record.
///
/// More information:
///  - [ECMA reference][spec]
///
///  [spec]: https://tc39.es/ecma262/#sec-ifabruptcloseiterator
macro_rules! if_abrupt_close_iterator {
    ($value:expr, $iterator_record:expr, $context:expr) => {
        match $value {
            // 1. If value is an abrupt completion, return ? IteratorClose(iteratorRecord, value).
            Err(err) => return $iterator_record.close(Err(err), $context),
            // 2. Else if value is a Completion Record, set value to value.
            Ok(value) => value,
        }
    };
}

// Export macro to crate level
pub(crate) use if_abrupt_close_iterator;

use super::OrdinaryObject;

/// `SetterThatIgnoresPrototypeProperties ( home, property )`
///
/// The abstract operation `SetterThatIgnoresPrototypeProperties` takes arguments
/// `home` (an Object) and `property` (a property key) and returns a function object.
/// It creates a setter function that only sets properties on own objects, ignoring
/// prototype properties.
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-setterthatignoresprototypeproperties
#[allow(dead_code)]
pub(crate) fn setter_that_ignores_prototype_properties(
    this: &JsValue,
    home: &JsObject,
    p: PropertyKey,
    v: JsValue,
    context: &mut Context,
) -> JsResult<JsValue> {
    // 1. If this is not an Object, then
    let this_obj = this.as_object().ok_or_else(|| {
        // a. Throw a TypeError exception.
        JsNativeError::typ().with_message("this is not an object")
    })?;

    // 2. If this is home, then
    if JsObject::equals(&this_obj, home) {
        // a. NOTE: Throwing here emulates assignment to a non-writable data property on the home object
        //    in strict mode code.
        // b. Throw a TypeError exception.
        return Err(JsNativeError::typ()
            .with_message("Cannot set property on home object")
            .into());
    }

    // 3. Let desc be ? this.[[GetOwnProperty]](p).
    let desc = this_obj.borrow().properties().get(&p);

    // 4. If desc is undefined, then
    if desc.is_none() {
        // a. Perform ? CreateDataPropertyOrThrow(this, p, v).
        this_obj.create_data_property_or_throw(p, v, context)?;
    } else {
        // 5. Else,
        // a. Perform ? Set(this, p, v, true).
        this_obj.set(p, v, true, context)?;
    }

    // 6. Return unused.
    Ok(JsValue::undefined())
}

/// The built-in iterator prototypes.
#[derive(Debug, Trace, Finalize)]
pub struct IteratorPrototypes {
    /// The `IteratorPrototype` object.
    iterator: JsObject,

    /// The `AsyncIteratorPrototype` object.
    async_iterator: JsObject,

    /// The `AsyncFromSyncIteratorPrototype` prototype object.
    async_from_sync_iterator: JsObject,

    /// The `ArrayIteratorPrototype` prototype object.
    array: JsObject,

    /// The `SetIteratorPrototype` prototype object.
    set: JsObject,

    /// The `StringIteratorPrototype` prototype object.
    string: JsObject,

    /// The `RegExpStringIteratorPrototype` prototype object.
    regexp_string: JsObject,

    /// The `MapIteratorPrototype` prototype object.
    map: JsObject,

    /// The `ForInIteratorPrototype` prototype object.
    for_in: JsObject,

    /// The `%WrapForValidIteratorPrototype%` prototype object.
    wrap_for_valid_iterator: JsObject,

    /// The `%IteratorHelperPrototype%` prototype object.
    iterator_helper: JsObject,

    /// The `%SegmentIteratorPrototype%` prototype object.
    #[cfg(feature = "intl")]
    segment: JsObject,
}

impl Default for IteratorPrototypes {
    fn default() -> Self {
        Self {
            iterator: JsObject::with_null_proto(),
            async_iterator: JsObject::with_null_proto(),
            async_from_sync_iterator: JsObject::with_null_proto(),
            array: JsObject::with_null_proto(),
            set: JsObject::with_null_proto(),
            string: JsObject::with_null_proto(),
            regexp_string: JsObject::with_null_proto(),
            map: JsObject::with_null_proto(),
            for_in: JsObject::with_null_proto(),
            wrap_for_valid_iterator: JsObject::with_null_proto(),
            iterator_helper: JsObject::with_null_proto(),
            #[cfg(feature = "intl")]
            segment: JsObject::with_null_proto(),
        }
    }
}

impl IteratorPrototypes {
    /// Returns the `ArrayIteratorPrototype` object.
    #[inline]
    #[must_use]
    pub fn array(&self) -> JsObject {
        self.array.clone()
    }

    /// Returns the `IteratorPrototype` object.
    #[inline]
    #[must_use]
    pub fn iterator(&self) -> JsObject {
        self.iterator.clone()
    }

    /// Returns the `AsyncIteratorPrototype` object.
    #[inline]
    #[must_use]
    pub fn async_iterator(&self) -> JsObject {
        self.async_iterator.clone()
    }

    /// Returns the `AsyncFromSyncIteratorPrototype` object.
    #[inline]
    #[must_use]
    pub fn async_from_sync_iterator(&self) -> JsObject {
        self.async_from_sync_iterator.clone()
    }

    /// Returns the `SetIteratorPrototype` object.
    #[inline]
    #[must_use]
    pub fn set(&self) -> JsObject {
        self.set.clone()
    }

    /// Returns the `StringIteratorPrototype` object.
    #[inline]
    #[must_use]
    pub fn string(&self) -> JsObject {
        self.string.clone()
    }

    /// Returns the `RegExpStringIteratorPrototype` object.
    #[inline]
    #[must_use]
    pub fn regexp_string(&self) -> JsObject {
        self.regexp_string.clone()
    }

    /// Returns the `MapIteratorPrototype` object.
    #[inline]
    #[must_use]
    pub fn map(&self) -> JsObject {
        self.map.clone()
    }

    /// Returns the `ForInIteratorPrototype` object.
    #[inline]
    #[must_use]
    pub fn for_in(&self) -> JsObject {
        self.for_in.clone()
    }

    /// Returns the `%WrapForValidIteratorPrototype%` object.
    #[inline]
    #[must_use]
    pub fn wrap_for_valid_iterator(&self) -> JsObject {
        self.wrap_for_valid_iterator.clone()
    }

    /// Returns the `%IteratorHelperPrototype%` object.
    #[inline]
    #[must_use]
    pub fn iterator_helper(&self) -> JsObject {
        self.iterator_helper.clone()
    }

    /// Returns the `%SegmentIteratorPrototype%` object.
    #[inline]
    #[must_use]
    #[cfg(feature = "intl")]
    pub fn segment(&self) -> JsObject {
        self.segment.clone()
    }
}

/// `GetIteratorDirect ( obj )`
///
/// The abstract operation `GetIteratorDirect` takes argument `obj` (an Object) and returns
/// either a normal completion containing an Iterator Record or a throw completion.
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-getiteratordirect
pub(crate) fn get_iterator_direct(
    obj: JsObject,
    context: &mut Context,
) -> JsResult<IteratorRecord> {
    // 1. Let nextMethod be ? Get(obj, "next").
    let next_method = obj.get(js_string!("next"), context)?;

    // 2. Let iteratorRecord be the Iterator Record { [[Iterator]]: obj, [[NextMethod]]: nextMethod, [[Done]]: false }.
    // 3. Return iteratorRecord.
    Ok(IteratorRecord::new(obj, next_method))
}

/// String handling mode for `GetIteratorFlattenable`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StringHandling {
    /// Iterate over strings.
    IterateStrings,

    /// Reject strings.
    RejectStrings,
}

/// `GetIteratorFlattenable ( obj, stringHandling )`
///
/// The abstract operation `GetIteratorFlattenable` takes arguments `obj` (an ECMAScript language value)
/// and `stringHandling` (iterate-strings or reject-strings) and returns either a normal completion
/// containing an Iterator Record or a throw completion.
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-getiteratorflattenable
pub(crate) fn get_iterator_flattenable(
    obj: &JsValue,
    string_handling: StringHandling,
    context: &mut Context,
) -> JsResult<IteratorRecord> {
    // 1. If obj is not an Object, then
    if !obj.is_object() {
        // a. If stringHandling is reject-strings or obj is not a String, throw a TypeError exception.
        if string_handling == StringHandling::RejectStrings || !obj.is_string() {
            return Err(JsNativeError::typ()
                .with_message("value is not an object or string")
                .into());
        }
    }

    // 2. Let method be ? GetMethod(obj, @@iterator).
    let method = obj.get_method(JsSymbol::iterator(), context)?;

    let iterator = if let Some(method) = method {
        // 4. Else,
        //     a. Let iterator be ? Call(method, obj).
        method.call(obj, &[], context)?
    } else {
        // 3. If method is undefined, then
        //     a. Let iterator be obj.
        obj.clone()
    };

    // 5. If iterator is not an Object, throw a TypeError exception.
    let iterator = iterator
        .as_object()
        .ok_or_else(|| JsNativeError::typ().with_message("returned iterator is not an object"))?;

    // 6. Return ? GetIteratorDirect(iterator).
    get_iterator_direct(iterator, context)
}

/// `%IteratorPrototype%` object
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-iterator-constructor
pub(crate) struct Iterator;

impl IntrinsicObject for Iterator {
    fn init(realm: &Realm) {
        let get_to_string_tag = BuiltInBuilder::callable(realm, Self::get_to_string_tag)
            .name(js_string!("get [Symbol.toStringTag]"))
            .build();

        let set_to_string_tag = BuiltInBuilder::callable(realm, Self::set_to_string_tag)
            .name(js_string!("set [Symbol.toStringTag]"))
            .build();

        BuiltInBuilder::from_standard_constructor::<Self>(realm)
            .method(Self::iterator, JsSymbol::iterator(), 0)
            .method(Self::to_array, js_string!("toArray"), 0)
            .method(Self::some, js_string!("some"), 1)
            .method(Self::for_each, js_string!("forEach"), 1)
            .method(Self::find, js_string!("find"), 1)
            .method(Self::every, js_string!("every"), 1)
            .method(Self::map, js_string!("map"), 1)
            .method(Self::concat, js_string!("concat"), 0)
            .method(Self::drop, js_string!("drop"), 1)
            .method(Self::take, js_string!("take"), 1)
            .method(Self::filter, js_string!("filter"), 1)
            .accessor(
                JsSymbol::to_string_tag(),
                Some(get_to_string_tag),
                Some(set_to_string_tag),
                Attribute::CONFIGURABLE,
            )
            .static_method(Self::from, js_string!("from"), 1)
            .build();
    }

    fn get(intrinsics: &Intrinsics) -> JsObject {
        Self::STANDARD_CONSTRUCTOR(intrinsics.constructors()).constructor()
    }
}

impl BuiltInObject for Iterator {
    const NAME: JsString = StaticJsStrings::ITERATOR;
}

impl BuiltInConstructor for Iterator {
    const CONSTRUCTOR_ARGUMENTS: usize = 0;
    const PROTOTYPE_STORAGE_SLOTS: usize = 13;
    const CONSTRUCTOR_STORAGE_SLOTS: usize = 1;

    const STANDARD_CONSTRUCTOR: fn(&StandardConstructors) -> &StandardConstructor =
        StandardConstructors::iterator;

    /// `Iterator ( )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator
    fn constructor(
        new_target: &JsValue,
        _args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        // 1. If NewTarget is undefined or the active function object, throw a TypeError exception.

        if new_target.is_undefined() {
            return Err(JsNativeError::typ()
                .with_message("Iterator constructor cannot be called without `new`")
                .into());
        }

        // Check if NewTarget is the active function object (Iterator constructor itself)
        if let (Some(new_target_obj), Some(active_fn)) =
            (new_target.as_object(), context.active_function_object())
        {
            if JsObject::equals(&new_target_obj, &active_fn) {
                return Err(JsNativeError::typ()
                    .with_message("Abstract class Iterator not directly constructable")
                    .into());
            }
        }

        // 2. Return ? OrdinaryCreateFromConstructor(NewTarget, "%Iterator.prototype%").
        let prototype =
            get_prototype_from_constructor(new_target, StandardConstructors::iterator, context)?;

        Ok(
            JsObject::from_proto_and_data_with_shared_shape(context.root_shape(), prototype, ())
                .into(),
        )
    }
}

impl Iterator {
    /// `Iterator.prototype [ %Symbol.iterator% ] ( )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype-%symbol.iterator%
    fn iterator(this: &JsValue, _: &[JsValue], _context: &mut Context) -> JsResult<JsValue> {
        // 1. Return the this value.
        Ok(this.clone())
    }

    /// `get Iterator.prototype [ %Symbol.toStringTag% ]`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-get-iterator.prototype-%symbol.tostringtag%
    fn get_to_string_tag(_: &JsValue, _: &[JsValue], _: &mut Context) -> JsResult<JsValue> {
        // 1. Return "Iterator".
        Ok(js_string!("Iterator").into())
    }

    /// `set Iterator.prototype [ %Symbol.toStringTag% ]`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-set-iterator.prototype-%symbol.tostringtag%
    fn set_to_string_tag(
        this: &JsValue,
        args: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        // 1. Let v be the first argument.
        let v = args.get_or_undefined(0).clone();
        // 2. Perform ? SetterThatIgnoresPrototypeProperties(this, %Iterator.prototype%, %Symbol.toStringTag%, v).
        let home = context.intrinsics().constructors().iterator().prototype();
        setter_that_ignores_prototype_properties(
            this,
            &home,
            JsSymbol::to_string_tag().into(),
            v,
            context,
        )
        // 3. Return undefined.
    }

    /// `Iterator.from ( O )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.from
    fn from(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        let o = args.get_or_undefined(0);

        // 2. Let iteratorRecord be ? GetIteratorFlattenable(O, iterate-strings).
        let iterator_record = get_iterator_flattenable(o, StringHandling::IterateStrings, context)?;

        // 3. Let hasInstance be ? OrdinaryHasInstance(%Iterator%, iteratorRecord.[[Iterator]]).
        let iterator_constructor = context.intrinsics().constructors().iterator().constructor();
        let has_instance = JsValue::ordinary_has_instance(
            &iterator_constructor.into(),
            &iterator_record.iterator().clone().into(),
            context,
        )?;

        // 4. If hasInstance is true, then
        if has_instance {
            // a. Return iteratorRecord.[[Iterator]].
            return Ok(iterator_record.iterator().clone().into());
        }

        // 5. Let wrapper be OrdinaryObjectCreate(%WrapForValidIteratorPrototype%, « [[Iterated]] »).
        // 6. Set wrapper.[[Iterated]] to iteratorRecord.
        // 7. Return wrapper.
        let wrap_proto = context
            .intrinsics()
            .objects()
            .iterator_prototypes()
            .wrap_for_valid_iterator();
        let wrapper = JsObject::from_proto_and_data_with_shared_shape(
            context.root_shape(),
            wrap_proto,
            WrapForValidIterator::new(iterator_record),
        );
        Ok(wrapper.into())
    }

    /// `Iterator.prototype.toArray ( )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.toarray
    fn to_array(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        let o = this;

        // 2. Let iteratorRecord be ? GetIteratorDirect(O).
        let iterator_record = get_iterator_direct(
            o.as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?,
            context,
        )?;

        // 3. Let list be ? IteratorToList(iteratorRecord).
        let list = iterator_record.into_list(context)?;

        // 4. Return CreateArrayFromList(list).
        Ok(Array::create_array_from_list(list.into_iter(), context).into())
    }

    /// `Iterator.prototype.some ( predicate )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.some
    fn some(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        let o = this;

        // 2. Let iteratorRecord be ? GetIteratorDirect(O).
        let mut iterator_record = get_iterator_direct(
            o.as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?,
            context,
        )?;

        // 3. If IsCallable(predicate) is false, throw a TypeError exception.
        let predicate = args
            .get_or_undefined(0)
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("predicate is not callable"))?;

        // 4. Let counter be 0.
        let mut counter = 0u64;

        // 5. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iteratorRecord).
            let value = iterator_record.step_value(context);

            // b. If value is done, return false.
            let value = if_abrupt_close_iterator!(value, iterator_record, context);
            let Some(value) = value else {
                return Ok(false.into());
            };

            // c. Let result be Completion(Call(predicate, undefined, « value, 𝔽(counter) »)).
            let result = predicate.call(&JsValue::undefined(), &[value, counter.into()], context);

            // d. IfAbruptCloseIterator(result, iteratorRecord).
            let result = if_abrupt_close_iterator!(result, iterator_record, context);

            // e. If ToBoolean(result) is true, return ? IteratorClose(iteratorRecord, NormalCompletion(true)).
            if result.to_boolean() {
                return iterator_record.close(Ok(true.into()), context);
            }

            // f. Set counter to counter + 1.
            counter += 1;
        }
    }

    /// `Iterator.prototype.forEach ( procedure )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.foreach
    fn for_each(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        let o = this;

        // 2. Let iteratorRecord be ? GetIteratorDirect(O).
        let mut iterator_record = get_iterator_direct(
            o.as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?,
            context,
        )?;

        // 3. If IsCallable(procedure) is false, throw a TypeError exception.
        let procedure = args.get_or_undefined(0);
        let procedure_fn = procedure
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("procedure is not callable"))?;

        // 4. Let counter be 0.
        let mut counter = 0u64;

        // 5. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iteratorRecord).
            let value = iterator_record.step_value(context);

            // b. If value is done, return undefined.
            let value = if_abrupt_close_iterator!(value, iterator_record, context);
            let Some(value) = value else {
                return Ok(JsValue::undefined());
            };

            // c. Let result be Completion(Call(procedure, undefined, « value, 𝔽(counter) »)).
            let result =
                procedure_fn.call(&JsValue::undefined(), &[value, counter.into()], context);

            // d. IfAbruptCloseIterator(result, iteratorRecord).
            let _result = if_abrupt_close_iterator!(result, iterator_record, context);

            // e. Set counter to counter + 1.
            counter += 1;
        }
    }

    /// `Iterator.prototype.find ( predicate )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.find
    fn find(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        let o = this;

        // 2. Let iteratorRecord be ? GetIteratorDirect(O).
        let mut iterator_record = get_iterator_direct(
            o.as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?,
            context,
        )?;

        // 3. If IsCallable(predicate) is false, throw a TypeError exception.
        let predicate = args
            .get_or_undefined(0)
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("predicate is not callable"))?;

        // 4. Let counter be 0.
        let mut counter = 0u64;

        // 5. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iteratorRecord).
            let value = iterator_record.step_value(context);

            // b. If value is done, return undefined.
            let value = if_abrupt_close_iterator!(value, iterator_record, context);
            let Some(value) = value else {
                return Ok(JsValue::undefined());
            };

            // c. Let result be Completion(Call(predicate, undefined, « value, 𝔽(counter) »)).
            let result = predicate.call(
                &JsValue::undefined(),
                &[value.clone(), counter.into()],
                context,
            );

            // d. IfAbruptCloseIterator(result, iteratorRecord).
            let result = if_abrupt_close_iterator!(result, iterator_record, context);

            // e. If ToBoolean(result) is true, return ? IteratorClose(iteratorRecord, NormalCompletion(value)).
            if result.to_boolean() {
                return iterator_record.close(Ok(value), context);
            }

            // f. Set counter to counter + 1.
            counter += 1;
        }
    }

    /// `Iterator.prototype.every ( predicate )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.every
    fn every(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        let o = this;

        // 2. Let iteratorRecord be ? GetIteratorDirect(O).
        let mut iterator_record = get_iterator_direct(
            o.as_object()
                .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?,
            context,
        )?;

        // 3. If IsCallable(predicate) is false, throw a TypeError exception.
        let predicate = args
            .get_or_undefined(0)
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("predicate is not callable"))?;

        // 4. Let counter be 0.
        let mut counter = 0u64;

        // 5. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iteratorRecord).
            let value = iterator_record.step_value(context);

            // b. If value is done, return true.
            let value = if_abrupt_close_iterator!(value, iterator_record, context);
            let Some(value) = value else {
                return Ok(true.into());
            };

            // c. Let result be Completion(Call(predicate, undefined, « value, 𝔽(counter) »)).
            let result = predicate.call(&JsValue::undefined(), &[value, counter.into()], context);

            // d. IfAbruptCloseIterator(result, iteratorRecord).
            let result = if_abrupt_close_iterator!(result, iterator_record, context);

            // e. If ToBoolean(result) is false, return ? IteratorClose(iteratorRecord, NormalCompletion(false)).
            if !result.to_boolean() {
                return iterator_record.close(Ok(false.into()), context);
            }

            // f. Set counter to counter + 1.
            counter += 1;
        }
    }

    /// `Iterator.prototype.map ( mapper )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.map
    fn map(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        let o = this;

        // 2. If O is not an Object, throw a TypeError exception.
        let o_obj = o
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?;

        // 3. If IsCallable(mapper) is false, throw a TypeError exception.
        let mapper = args
            .get_or_undefined(0)
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("mapper is not callable"))?;

        // 4. Let iterated be ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(o_obj, context)?;

        // 5. Let result be CreateIteratorFromClosure(closure, "Iterator Helper", %IteratorHelperPrototype%, « [[UnderlyingIterator]] »).
        // 6. Set result.[[UnderlyingIterator]] to iterated.
        let result = IteratorHelper::create(
            iterated,
            IteratorHelperKind::Map {
                mapper: mapper.clone(),
            },
            context,
        );

        // 7. Return result.
        Ok(result.into())
    }

    /// `Iterator.prototype.concat ( ...items )`
    ///
    /// More information:
    ///  - [ECMAScript reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.concat
    fn concat(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        let o = this;

        // 2. Let iteratorRecord be ? GetIteratorDirect(O).
        let o_obj = o
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?;

        let iterator_record = get_iterator_direct(o_obj, context)?;

        // 3. Let iterables be a new empty List.
        let mut iterables = Vec::new();
        iterables.push(iterator_record);

        // 4. For each element item of items, do
        for item in args.iter() {
            // a. If item is an Object, then
            if item.is_object() {
                // i. Let iteratorRecord be ? GetIteratorFlattenable(item, reject-strings).
                let iter_record =
                    get_iterator_flattenable(item, StringHandling::RejectStrings, context)?;
                // ii. Append iteratorRecord to iterables.
                iterables.push(iter_record);
            } else {
                // b. Else,
                // i. Let iteratorRecord be ? GetIteratorFlattenable(item, iterate-strings).
                let iter_record =
                    get_iterator_flattenable(item, StringHandling::IterateStrings, context)?;
                // ii. Append iteratorRecord to iterables.
                iterables.push(iter_record);
            }
        }

        // 5. Let result be CreateIteratorFromClosure(closure, "Iterator Helper", %IteratorHelperPrototype%, « [[UnderlyingIterators]] »).
        // 6. Set result.[[UnderlyingIterators]] to iterables.
        let result = IteratorHelper::create_concat(iterables, context);

        // 7. Return result.
        Ok(result.into())
    }

    /// `Iterator.prototype.drop ( limit )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.drop
    fn drop(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let o_obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?;

        // 3. Let numLimit be ? ToNumber(limit).
        let limit = args.get_or_undefined(0);
        let num_limit = limit.to_number(context)?;

        // 4. If numLimit is NaN, throw a RangeError exception.
        if num_limit.is_nan() {
            return Err(JsNativeError::range()
                .with_message("limit must not be NaN")
                .into());
        }

        // 5. Let integerLimit be ! ToIntegerOrInfinity(numLimit).
        let integer_limit = limit.to_integer_or_infinity(context)?;

        // 6. If integerLimit < 0, throw a RangeError exception.
        match integer_limit {
            IntegerOrInfinity::Integer(i) if i < 0 => {
                return Err(JsNativeError::range()
                    .with_message("limit must be non-negative")
                    .into());
            }
            IntegerOrInfinity::NegativeInfinity => {
                return Err(JsNativeError::range()
                    .with_message("limit must be non-negative")
                    .into());
            }
            _ => {}
        }

        // 7. Let iterated be ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(o_obj.clone(), context)?;

        // 8. Let result be CreateIteratorFromClosure(closure, "Iterator Helper", %IteratorHelperPrototype%, « [[UnderlyingIterator]] »).
        let result = IteratorHelper::create(
            iterated,
            IteratorHelperKind::Drop {
                remaining: integer_limit,
            },
            context,
        );

        // 9. Return result.
        Ok(result.into())
    }

    /// `Iterator.prototype.take ( limit )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.take
    fn take(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let o_obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?;

        // 3. Let numLimit be ? ToNumber(limit).
        let limit = args.get_or_undefined(0);
        let num_limit = limit.to_number(context)?;

        // 4. If numLimit is NaN, throw a RangeError exception.
        if num_limit.is_nan() {
            return Err(JsNativeError::range()
                .with_message("limit must not be NaN")
                .into());
        }

        // 5. Let integerLimit be ! ToIntegerOrInfinity(numLimit).
        let integer_limit = limit.to_integer_or_infinity(context)?;

        // 6. If integerLimit < 0, throw a RangeError exception.
        match integer_limit {
            IntegerOrInfinity::Integer(i) if i < 0 => {
                return Err(JsNativeError::range()
                    .with_message("limit must be non-negative")
                    .into());
            }
            IntegerOrInfinity::NegativeInfinity => {
                return Err(JsNativeError::range()
                    .with_message("limit must be non-negative")
                    .into());
            }
            _ => {}
        }

        // 7. Let iterated be ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(o_obj.clone(), context)?;

        // 8. Let result be CreateIteratorFromClosure(closure, "Iterator Helper", %IteratorHelperPrototype%, « [[UnderlyingIterator]] »).
        let result = IteratorHelper::create(
            iterated,
            IteratorHelperKind::Take {
                remaining: integer_limit,
            },
            context,
        );

        // 9. Return result.
        Ok(result.into())
    }

    /// `Iterator.prototype.filter ( predicate )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iterator.prototype.filter
    fn filter(this: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let o_obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?;

        // 3. If IsCallable(predicate) is false, throw a TypeError exception.
        let predicate = args.get_or_undefined(0);
        let predicate_fn = predicate
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("predicate is not callable"))?;

        // 4. Let iterated be ? GetIteratorDirect(O).
        let iterated = get_iterator_direct(o_obj.clone(), context)?;

        // 5. Let result be CreateIteratorFromClosure(closure, "Iterator Helper", %IteratorHelperPrototype%, « [[UnderlyingIterator]] »).
        let result = IteratorHelper::create(
            iterated,
            IteratorHelperKind::Filter {
                predicate: predicate_fn.clone(),
            },
            context,
        );

        // 6. Return result.
        Ok(result.into())
    }
}

/// The kind of iterator helper.
#[derive(Debug, Clone, Finalize, Trace)]
enum IteratorHelperKind {
    /// Map helper: applies a mapper function to each value.
    Map {
        /// The mapper function.
        mapper: JsObject,
    },
    /// Concat helper: concatenates multiple iterables.
    Concat,
    /// Drop helper: skips the first `remaining` items.
    Drop {
        /// Number of items remaining to skip.
        #[unsafe_ignore_trace]
        remaining: IntegerOrInfinity,
    },
    /// Take helper: yields at most `remaining` items.
    Take {
        /// Number of items remaining to take.
        #[unsafe_ignore_trace]
        remaining: IntegerOrInfinity,
    },
    /// Filter helper: filters values based on a predicate.
    Filter {
        /// The predicate function.
        predicate: JsObject,
    },
}

/// The `IteratorHelper` object.
///
/// This object wraps an iterator and applies helper operations.
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-iteratorhelperprototype-object
#[derive(Debug, Clone, Finalize, Trace, JsData)]
pub(crate) struct IteratorHelper {
    /// The `[[UnderlyingIterator]]` internal slot.
    #[unsafe_ignore_trace]
    underlying_iterator: Option<IteratorRecord>,

    /// The list of iterator records for Concat helper.
    #[unsafe_ignore_trace]
    iterables: Vec<IteratorRecord>,

    /// The kind of iterator helper.
    kind: IteratorHelperKind,

    /// The counter for tracking iterations.
    counter: u64,
}

impl IteratorHelper {
    /// Creates a new `IteratorHelper`.
    fn create(
        underlying_iterator: IteratorRecord,
        kind: IteratorHelperKind,
        context: &mut Context,
    ) -> JsObject {
        let iterator_helper = Self {
            underlying_iterator: Some(underlying_iterator),
            iterables: Vec::new(),
            kind,
            counter: 0,
        };

        JsObject::from_proto_and_data_with_shared_shape::<_, IteratorHelper>(
            context.root_shape(),
            context
                .intrinsics()
                .objects()
                .iterator_prototypes()
                .iterator_helper(),
            iterator_helper,
        )
        .upcast()
    }

    /// Creates a new `IteratorHelper` for Concat with multiple iterables.
    fn create_concat(iterables: Vec<IteratorRecord>, context: &mut Context) -> JsObject {
        let iterator_helper = Self {
            underlying_iterator: None,
            iterables,
            kind: IteratorHelperKind::Concat,
            counter: 0,
        };

        JsObject::from_proto_and_data_with_shared_shape::<_, IteratorHelper>(
            context.root_shape(),
            context
                .intrinsics()
                .objects()
                .iterator_prototypes()
                .iterator_helper(),
            iterator_helper,
        )
        .upcast()
    }

    /// Gets the `[[UnderlyingIterator]]` internal slot.
    fn underlying_iterator(&mut self) -> &mut IteratorRecord {
        self.underlying_iterator
            .as_mut()
            .expect("underlying_iterator is None for Concat helper")
    }
}

/// The `%IteratorHelperPrototype%` object.
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-iteratorhelperprototype-object
pub(crate) struct IteratorHelperPrototype;

impl IntrinsicObject for IteratorHelperPrototype {
    fn init(realm: &Realm) {
        BuiltInBuilder::with_intrinsic::<Self>(realm)
            .prototype(
                realm
                    .intrinsics()
                    .objects()
                    .iterator_prototypes()
                    .iterator(),
            )
            .static_method(Self::next, js_string!("next"), 0)
            .static_method(Self::r#return, js_string!("return"), 0)
            .build();
    }

    fn get(intrinsics: &Intrinsics) -> JsObject {
        intrinsics.objects().iterator_prototypes().iterator_helper()
    }
}

impl IteratorHelperPrototype {
    /// `%IteratorHelperPrototype%.next ( )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-%iteratorhelperprototype%.next
    fn next(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[UnderlyingIterator]]).
        let o_obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?;

        let mut iterator_helper = o_obj.downcast_mut::<IteratorHelper>().ok_or_else(|| {
            JsNativeError::typ().with_message("this is not an IteratorHelper object")
        })?;

        // Process based on the kind of helper.
        let kind = iterator_helper.kind.clone();

        match kind {
            IteratorHelperKind::Map { ref mapper } => {
                // Get next value from underlying iterator.
                let value = iterator_helper.underlying_iterator().step_value(context)?;

                if let Some(value) = value {
                    let counter = iterator_helper.counter;
                    iterator_helper.counter += 1;

                    // Drop the mutable borrow before calling the mapper.
                    let mapper = mapper.clone();
                    drop(iterator_helper);

                    // Apply the mapper function.
                    let mapped =
                        mapper.call(&JsValue::undefined(), &[value, counter.into()], context)?;

                    // Return the mapped value.
                    Ok(create_iter_result_object(mapped, false, context))
                } else {
                    // Iterator is done.
                    Ok(create_iter_result_object(
                        JsValue::undefined(),
                        true,
                        context,
                    ))
                }
            }
            IteratorHelperKind::Concat => {
                // Concat helper implementation.
                // Keep trying iterables until we find a value or run out.
                loop {
                    if iterator_helper.iterables.is_empty() {
                        // No more iterables to process.
                        return Ok(create_iter_result_object(
                            JsValue::undefined(),
                            true,
                            context,
                        ));
                    }

                    // Get the current iterable (first in the list).
                    let value = iterator_helper.iterables[0].step_value(context)?;

                    if let Some(value) = value {
                        // Found a value, return it.
                        return Ok(create_iter_result_object(value, false, context));
                    } else {
                        // Current iterable is exhausted, move to the next one.
                        iterator_helper.iterables.remove(0);
                    }
                }
            }
            IteratorHelperKind::Drop { ref remaining } => {
                // Drop helper implementation.
                let mut remaining = *remaining;

                // Skip items while remaining > 0.
                loop {
                    match remaining {
                        IntegerOrInfinity::Integer(0) => {
                            // Done skipping, return the next value.
                            let value =
                                iterator_helper.underlying_iterator().step_value(context)?;

                            return if let Some(value) = value {
                                Ok(create_iter_result_object(value, false, context))
                            } else {
                                Ok(create_iter_result_object(
                                    JsValue::undefined(),
                                    true,
                                    context,
                                ))
                            };
                        }
                        IntegerOrInfinity::PositiveInfinity => {
                            // Skip forever, always return done.
                            return Ok(create_iter_result_object(
                                JsValue::undefined(),
                                true,
                                context,
                            ));
                        }
                        IntegerOrInfinity::Integer(n) => {
                            // Skip one item and decrement.
                            let value =
                                iterator_helper.underlying_iterator().step_value(context)?;

                            if value.is_none() {
                                // Iterator exhausted before we finished skipping.
                                return Ok(create_iter_result_object(
                                    JsValue::undefined(),
                                    true,
                                    context,
                                ));
                            }

                            // Decrement remaining.
                            remaining = IntegerOrInfinity::Integer(n - 1);
                            iterator_helper.kind = IteratorHelperKind::Drop { remaining };
                        }
                        IntegerOrInfinity::NegativeInfinity => {
                            unreachable!("drop with negative infinity should have been rejected")
                        }
                    }
                }
            }
            IteratorHelperKind::Take { ref remaining } => {
                // Take helper implementation.
                let remaining = *remaining;

                match remaining {
                    IntegerOrInfinity::Integer(0) => {
                        // No more items to take, iterator is done.
                        Ok(create_iter_result_object(
                            JsValue::undefined(),
                            true,
                            context,
                        ))
                    }
                    IntegerOrInfinity::PositiveInfinity => {
                        // Take infinity, just pass through all values.
                        let value = iterator_helper.underlying_iterator().step_value(context)?;

                        if let Some(value) = value {
                            Ok(create_iter_result_object(value, false, context))
                        } else {
                            Ok(create_iter_result_object(
                                JsValue::undefined(),
                                true,
                                context,
                            ))
                        }
                    }
                    IntegerOrInfinity::Integer(n) => {
                        // Take one item and decrement.
                        let value = iterator_helper.underlying_iterator().step_value(context)?;

                        if let Some(value) = value {
                            // Decrement remaining.
                            iterator_helper.kind = IteratorHelperKind::Take {
                                remaining: IntegerOrInfinity::Integer(n - 1),
                            };
                            Ok(create_iter_result_object(value, false, context))
                        } else {
                            // Underlying iterator exhausted.
                            Ok(create_iter_result_object(
                                JsValue::undefined(),
                                true,
                                context,
                            ))
                        }
                    }
                    IntegerOrInfinity::NegativeInfinity => {
                        unreachable!("take with negative infinity should have been rejected")
                    }
                }
            }
            IteratorHelperKind::Filter { ref predicate } => {
                // Filter helper implementation.
                // Keep trying values until we find one that passes the predicate.
                loop {
                    let value = iterator_helper.underlying_iterator().step_value(context)?;

                    if let Some(value) = value {
                        let counter = iterator_helper.counter;
                        iterator_helper.counter += 1;

                        // Drop the mutable borrow before calling the predicate.
                        let predicate = predicate.clone();
                        drop(iterator_helper);

                        // Call the predicate function.
                        let selected = predicate.call(
                            &JsValue::undefined(),
                            &[value.clone(), counter.into()],
                            context,
                        )?;

                        // If the predicate returns truthy, return this value.
                        if selected.to_boolean() {
                            return Ok(create_iter_result_object(value, false, context));
                        }

                        // Otherwise, reacquire the borrow and continue.
                        iterator_helper =
                            o_obj.downcast_mut::<IteratorHelper>().ok_or_else(|| {
                                JsNativeError::typ()
                                    .with_message("this is not an IteratorHelper object")
                            })?;
                    } else {
                        // Iterator is done.
                        return Ok(create_iter_result_object(
                            JsValue::undefined(),
                            true,
                            context,
                        ));
                    }
                }
            }
        }
    }

    /// `%IteratorHelperPrototype%.return ( )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-%iteratorhelperprototype%.return
    fn r#return(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be the this value.
        // 2. Perform ? RequireInternalSlot(O, [[UnderlyingIterator]]).
        let o_obj = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("this is not an object"))?;

        let mut iterator_helper = o_obj.downcast_mut::<IteratorHelper>().ok_or_else(|| {
            JsNativeError::typ().with_message("this is not an IteratorHelper object")
        })?;

        // Handle different helper kinds.
        match &iterator_helper.kind {
            IteratorHelperKind::Concat => {
                // For Concat, we need to close all remaining iterators.
                let iterators = std::mem::take(&mut iterator_helper.iterables);
                drop(iterator_helper);

                for iter_record in iterators {
                    let iterator = iter_record.iterator();
                    let return_method = iterator.get_method(js_string!("return"), context)?;
                    if let Some(return_method) = return_method {
                        return_method.call(&iterator.clone().into(), &[], context)?;
                    }
                }

                Ok(create_iter_result_object(
                    JsValue::undefined(),
                    true,
                    context,
                ))
            }
            _ => {
                // 3. Let iterator be O.[[UnderlyingIterator]].[[Iterator]].
                let iterator = iterator_helper.underlying_iterator().iterator().clone();
                drop(iterator_helper);

                // 4. Assert: iterator is an Object.
                // 5. Let returnMethod be ? GetMethod(iterator, "return").
                let return_method = iterator.get_method(js_string!("return"), context)?;

                // 6. If returnMethod is undefined, then
                if return_method.is_none() {
                    // a. Return CreateIterResultObject(undefined, true).
                    return Ok(create_iter_result_object(
                        JsValue::undefined(),
                        true,
                        context,
                    ));
                }

                // 7. Return ? Call(returnMethod, iterator).
                return_method
                    .unwrap()
                    .call(&iterator.clone().into(), &[], context)
            }
        }
    }
}

/// The `WrapForValidIterator` object.
///
/// This object wraps an iterator to ensure it conforms to the Iterator interface.
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-wrapforvaliditeratorprototype-object
#[derive(Debug, Clone, Finalize, Trace, JsData)]
pub(crate) struct WrapForValidIterator {
    /// The `[[Iterated]]` internal slot.
    #[unsafe_ignore_trace]
    iterated: IteratorRecord,
}

impl WrapForValidIterator {
    /// Creates a new `WrapForValidIterator`.
    pub(crate) fn new(iterated: IteratorRecord) -> Self {
        Self { iterated }
    }

    /// Gets the iterator record.
    pub(crate) fn iterated(&self) -> &IteratorRecord {
        &self.iterated
    }
}

/// `%WrapForValidIteratorPrototype%` object.
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-wrapforvaliditeratorprototype-object
pub(crate) struct WrapForValidIteratorPrototype;

impl IntrinsicObject for WrapForValidIteratorPrototype {
    fn init(realm: &Realm) {
        BuiltInBuilder::with_intrinsic::<Self>(realm)
            .prototype(
                realm
                    .intrinsics()
                    .objects()
                    .iterator_prototypes()
                    .iterator(),
            )
            .static_method(Self::next, js_string!("next"), 0)
            .static_method(Self::return_fn, js_string!("return"), 0)
            .build();
    }

    fn get(intrinsics: &Intrinsics) -> JsObject {
        intrinsics
            .objects()
            .iterator_prototypes()
            .wrap_for_valid_iterator()
    }
}

impl WrapForValidIteratorPrototype {
    /// `%WrapForValidIteratorPrototype%.next ( )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-%wrapforvaliditeratorprototype%.next
    pub(crate) fn next(this: &JsValue, _: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
        // 1. Let O be this value.
        // 2. Perform ? RequireInternalSlot(O, [[Iterated]]).
        let o = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("`this` is not an object"))?;

        let wrap = o.downcast_mut::<WrapForValidIterator>().ok_or_else(|| {
            JsNativeError::typ().with_message("`this` does not have a [[Iterated]] internal slot")
        })?;

        // 3. Let iteratorRecord be O.[[Iterated]].
        let iterator_record = wrap.iterated.clone();
        drop(wrap);

        // 4. Return ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]]).
        let result = iterator_record
            .next_method()
            .as_callable()
            .ok_or_else(|| JsNativeError::typ().with_message("next method is not callable"))?
            .call(&iterator_record.iterator().clone().into(), &[], context)?;

        Ok(result)
    }

    /// `%WrapForValidIteratorPrototype%.return ( )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-%wrapforvaliditeratorprototype%.return
    pub(crate) fn return_fn(
        this: &JsValue,
        _: &[JsValue],
        context: &mut Context,
    ) -> JsResult<JsValue> {
        // 1. Let O be this value.
        // 2. Perform ? RequireInternalSlot(O, [[Iterated]]).
        let o = this
            .as_object()
            .ok_or_else(|| JsNativeError::typ().with_message("`this` is not an object"))?;

        let wrap = o.downcast_ref::<WrapForValidIterator>().ok_or_else(|| {
            JsNativeError::typ().with_message("`this` does not have a [[Iterated]] internal slot")
        })?;

        // 3. Let iterator be O.[[Iterated]].[[Iterator]].
        let iterator = wrap.iterated().iterator().clone();
        drop(wrap);

        // 4. Assert: iterator is an Object.
        // (guaranteed by the type system)

        // 5. Let returnMethod be ? GetMethod(iterator, "return").
        let return_method = iterator.get_method(js_string!("return"), context)?;

        // 6. If returnMethod is undefined, then
        if return_method.is_none() {
            //     a. Return CreateIterResultObject(undefined, true).
            return Ok(create_iter_result_object(
                JsValue::undefined(),
                true,
                context,
            ));
        }

        // 7. Return ? Call(returnMethod, iterator).
        let return_method = return_method.expect("return_method is Some");
        return_method.call(&iterator.into(), &[], context)
    }
}

/// `%AsyncIteratorPrototype%` object
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-asynciteratorprototype
pub(crate) struct AsyncIterator;

impl IntrinsicObject for AsyncIterator {
    fn init(realm: &Realm) {
        BuiltInBuilder::with_intrinsic::<Self>(realm)
            .static_method(|v, _, _| Ok(v.clone()), JsSymbol::async_iterator(), 0)
            .build();
    }

    fn get(intrinsics: &Intrinsics) -> JsObject {
        intrinsics.objects().iterator_prototypes().async_iterator()
    }
}

/// `CreateIterResultObject( value, done )`
///
/// Generates an object supporting the `IteratorResult` interface.
pub fn create_iter_result_object(value: JsValue, done: bool, context: &mut Context) -> JsValue {
    // 1. Assert: Type(done) is Boolean.
    // 2. Let obj be ! OrdinaryObjectCreate(%Object.prototype%).
    // 3. Perform ! CreateDataPropertyOrThrow(obj, "value", value).
    // 4. Perform ! CreateDataPropertyOrThrow(obj, "done", done).
    let obj = context
        .intrinsics()
        .templates()
        .iterator_result()
        .create(OrdinaryObject, vec![value, done.into()]);

    // 5. Return obj.
    obj.into()
}

/// Iterator hint for `GetIterator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IteratorHint {
    /// Hints that the iterator should be sync.
    Sync,

    /// Hints that the iterator should be async.
    Async,
}

impl JsValue {
    /// `GetIteratorFromMethod ( obj, method )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-getiteratorfrommethod
    pub fn get_iterator_from_method(
        &self,
        method: &JsObject,
        context: &mut Context,
    ) -> JsResult<IteratorRecord> {
        // 1. Let iterator be ? Call(method, obj).
        let iterator = method.call(self, &[], context)?;
        // 2. If iterator is not an Object, throw a TypeError exception.
        let iterator_obj = iterator.as_object().ok_or_else(|| {
            JsNativeError::typ().with_message("returned iterator is not an object")
        })?;
        // 3. Let nextMethod be ? Get(iterator, "next").
        let next_method = iterator_obj.get(js_string!("next"), context)?;
        // 4. Let iteratorRecord be the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: nextMethod, [[Done]]: false }.
        // 5. Return iteratorRecord.
        Ok(IteratorRecord::new(iterator_obj.clone(), next_method))
    }

    /// `GetIterator ( obj, kind )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-getiterator
    pub fn get_iterator(
        &self,
        hint: IteratorHint,
        context: &mut Context,
    ) -> JsResult<IteratorRecord> {
        let method = match hint {
            // 1. If kind is async, then
            IteratorHint::Async => {
                // a. Let method be ? GetMethod(obj, %Symbol.asyncIterator%).
                let Some(method) = self.get_method(JsSymbol::async_iterator(), context)? else {
                    // b. If method is undefined, then
                    //     i. Let syncMethod be ? GetMethod(obj, %Symbol.iterator%).
                    let sync_method =
                        self.get_method(JsSymbol::iterator(), context)?
                            .ok_or_else(|| {
                                // ii. If syncMethod is undefined, throw a TypeError exception.
                                JsNativeError::typ().with_message(format!(
                                    "value with type `{}` is not iterable",
                                    self.type_of()
                                ))
                            })?;
                    // iii. Let syncIteratorRecord be ? GetIteratorFromMethod(obj, syncMethod).
                    let sync_iterator_record =
                        self.get_iterator_from_method(&sync_method, context)?;
                    // iv. Return CreateAsyncFromSyncIterator(syncIteratorRecord).
                    return Ok(AsyncFromSyncIterator::create(sync_iterator_record, context));
                };

                Some(method)
            }
            // 2. Else,
            IteratorHint::Sync => {
                // a. Let method be ? GetMethod(obj, %Symbol.iterator%).
                self.get_method(JsSymbol::iterator(), context)?
            }
        };

        let method = method.ok_or_else(|| {
            // 3. If method is undefined, throw a TypeError exception.
            JsNativeError::typ().with_message(format!(
                "value with type `{}` is not iterable",
                self.type_of()
            ))
        })?;

        // 4. Return ? GetIteratorFromMethod(obj, method).
        self.get_iterator_from_method(&method, context)
    }
}

/// The result of the iteration process.
#[derive(Debug, Clone, Trace, Finalize)]
pub struct IteratorResult {
    object: JsObject,
}

impl IteratorResult {
    /// Gets a new `IteratorResult` from a value. Returns `Err` if
    /// the value is not a [`JsObject`]
    pub(crate) fn from_value(value: JsValue) -> JsResult<Self> {
        if let Some(object) = value.into_object() {
            Ok(Self { object })
        } else {
            Err(JsNativeError::typ()
                .with_message("next value should be an object")
                .into())
        }
    }

    /// Gets the inner object of this `IteratorResult`.
    pub(crate) const fn object(&self) -> &JsObject {
        &self.object
    }

    /// `IteratorComplete ( iterResult )`
    ///
    /// The abstract operation `IteratorComplete` takes argument `iterResult` (an `Object`) and
    /// returns either a normal completion containing a `Boolean` or a throw completion.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iteratorcomplete
    #[inline]
    pub fn complete(&self, context: &mut Context) -> JsResult<bool> {
        // 1. Return ToBoolean(? Get(iterResult, "done")).
        Ok(self.object.get(js_string!("done"), context)?.to_boolean())
    }

    /// `IteratorValue ( iterResult )`
    ///
    /// The abstract operation `IteratorValue` takes argument `iterResult` (an `Object`) and
    /// returns either a normal completion containing an ECMAScript language value or a throw
    /// completion.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iteratorvalue
    #[inline]
    pub fn value(&self, context: &mut Context) -> JsResult<JsValue> {
        // 1. Return ? Get(iterResult, "value").
        self.object.get(js_string!("value"), context)
    }
}

/// Iterator Record
///
/// An Iterator Record is a Record value used to encapsulate an
/// `Iterator` or `AsyncIterator` along with the `next` method.
///
/// More information:
///  - [ECMA reference][spec]
///
/// [spec]: https://tc39.es/ecma262/#sec-iterator-records
#[derive(Clone, Debug, Finalize, Trace)]
pub struct IteratorRecord {
    /// `[[Iterator]]`
    ///
    /// An object that conforms to the `Iterator` or `AsyncIterator` interface.
    iterator: JsObject,

    /// `[[NextMethod]]`
    ///
    /// The `next` method of the `[[Iterator]]` object.
    next_method: JsValue,

    /// `[[Done]]`
    ///
    /// Whether the iterator has been closed.
    done: bool,

    /// The result of the last call to `next`.
    last_result: IteratorResult,
}

impl IteratorRecord {
    /// Creates a new `IteratorRecord` with the given iterator object, next method and `done` flag.
    #[inline]
    #[must_use]
    pub fn new(iterator: JsObject, next_method: JsValue) -> Self {
        Self {
            iterator,
            next_method,
            done: false,
            last_result: IteratorResult {
                object: JsObject::with_null_proto(),
            },
        }
    }

    /// Get the `[[Iterator]]` field of the `IteratorRecord`.
    pub(crate) const fn iterator(&self) -> &JsObject {
        &self.iterator
    }

    /// Gets the `[[NextMethod]]` field of the `IteratorRecord`.
    pub(crate) const fn next_method(&self) -> &JsValue {
        &self.next_method
    }

    /// Gets the last result object of the iterator record.
    pub(crate) const fn last_result(&self) -> &IteratorResult {
        &self.last_result
    }

    /// Runs `f`, setting the `done` field of this `IteratorRecord` to `true` if `f` returns
    /// an error.
    fn set_done_on_err<R, F>(&mut self, f: F) -> JsResult<R>
    where
        F: FnOnce(&mut Self) -> JsResult<R>,
    {
        let result = f(self);
        if result.is_err() {
            self.done = true;
        }
        result
    }

    /// Gets the current value of the `IteratorRecord`.
    pub(crate) fn value(&mut self, context: &mut Context) -> JsResult<JsValue> {
        self.set_done_on_err(|iter| iter.last_result.value(context))
    }

    /// Get the `[[Done]]` field of the `IteratorRecord`.
    pub(crate) const fn done(&self) -> bool {
        self.done
    }

    /// Updates the current result value of this iterator record.
    pub(crate) fn update_result(&mut self, result: JsValue, context: &mut Context) -> JsResult<()> {
        self.set_done_on_err(|iter| {
            // 3. If Type(result) is not Object, throw a TypeError exception.
            // 4. Return result.
            // `IteratorResult::from_value` does this for us.

            // `IteratorStep(iteratorRecord)`
            // https://tc39.es/ecma262/#sec-iteratorstep

            // 1. Let result be ? IteratorNext(iteratorRecord).
            let result = IteratorResult::from_value(result)?;
            // 2. Let done be ? IteratorComplete(result).
            // 3. If done is true, return false.
            iter.done = result.complete(context)?;

            iter.last_result = result;

            Ok(())
        })
    }

    /// `IteratorNext ( iteratorRecord [ , value ] )`
    ///
    /// The abstract operation `IteratorNext` takes argument `iteratorRecord` (an `Iterator`
    /// Record) and optional argument `value` (an ECMAScript language value) and returns either a
    /// normal completion containing an `Object` or a throw completion.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iteratornext
    pub(crate) fn next(
        &mut self,
        value: Option<&JsValue>,
        context: &mut Context,
    ) -> JsResult<IteratorResult> {
        // 1. If value is not present, then
        //     a. Let result be Completion(Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]])).
        // 2. Else,
        //     a. Let result be Completion(Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]], « value »)).
        // 3. If result is a throw completion, then
        //     a. Set iteratorRecord.[[Done]] to true.
        //     b. Return ? result.
        // 4. Set result to ! result.
        // 5. If result is not an Object, then
        //     a. Set iteratorRecord.[[Done]] to true.
        //     b. Throw a TypeError exception.
        // 6. Return result.
        // NOTE: In this case, `set_done_on_err` does all the heavylifting for us, which
        // simplifies the instructions below.
        self.set_done_on_err(|iter| {
            iter.next_method
                .call(
                    &iter.iterator.clone().into(),
                    value.map_or(&[], std::slice::from_ref),
                    context,
                )
                .and_then(IteratorResult::from_value)
        })
    }

    /// `IteratorStep ( iteratorRecord )`
    ///
    /// Updates the `IteratorRecord` and returns `true` if the next result record returned
    /// `done: true`, otherwise returns `false`. This differs slightly from the spec, but also
    /// simplifies some logic around iterators.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iteratorstep
    pub(crate) fn step(&mut self, context: &mut Context) -> JsResult<bool> {
        self.set_done_on_err(|iter| {
            // 1. Let result be ? IteratorNext(iteratorRecord).
            let result = iter.next(None, context)?;

            // 2. Let done be Completion(IteratorComplete(result)).
            // 3. If done is a throw completion, then
            //     a. Set iteratorRecord.[[Done]] to true.
            //     b. Return ? done.
            // 4. Set done to ! done.
            // 5. If done is true, then
            //     a. Set iteratorRecord.[[Done]] to true.
            //     b. Return done.
            iter.done = result.complete(context)?;

            iter.last_result = result;

            // 6. Return result.
            Ok(iter.done)
        })
    }

    /// `IteratorStepValue ( iteratorRecord )`
    ///
    /// Updates the `IteratorRecord` and returns `Some(value)` if the next result record returned
    /// `done: true`, otherwise returns `None`.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    /// [spec]: https://tc39.es/ecma262/#sec-iteratorstepvalue
    pub(crate) fn step_value(&mut self, context: &mut Context) -> JsResult<Option<JsValue>> {
        // 1. Let result be ? IteratorStep(iteratorRecord).
        if self.step(context)? {
            // 2. If result is done, then
            //     a. Return done.
            Ok(None)
        } else {
            // 3. Let value be Completion(IteratorValue(result)).
            // 4. If value is a throw completion, then
            //     a. Set iteratorRecord.[[Done]] to true.
            // 5. Return ? value.
            self.value(context).map(Some)
        }
    }

    /// `IteratorClose ( iteratorRecord, completion )`
    ///
    /// The abstract operation `IteratorClose` takes arguments `iteratorRecord` (an
    /// [Iterator Record][Self]) and `completion` (a `Completion` Record) and returns a
    /// `Completion` Record. It is used to notify an iterator that it should perform any actions it
    /// would normally perform when it has reached its completed state.
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    ///  [spec]: https://tc39.es/ecma262/#sec-iteratorclose
    pub(crate) fn close(
        &self,
        completion: JsResult<JsValue>,
        context: &mut Context,
    ) -> JsResult<JsValue> {
        // 1. Assert: Type(iteratorRecord.[[Iterator]]) is Object.

        // 2. Let iterator be iteratorRecord.[[Iterator]].
        let iterator = &self.iterator;

        // 3. Let innerResult be Completion(GetMethod(iterator, "return")).
        let inner_result = iterator.get_method(js_string!("return"), context);

        // 4. If innerResult.[[Type]] is normal, then
        let inner_result = match inner_result {
            Ok(inner_result) => {
                // a. Let return be innerResult.[[Value]].
                let r#return = inner_result;

                if let Some(r#return) = r#return {
                    // c. Set innerResult to Completion(Call(return, iterator)).
                    r#return.call(&iterator.clone().into(), &[], context)
                } else {
                    // b. If return is undefined, return ? completion.
                    return completion;
                }
            }
            Err(inner_result) => {
                // 5. If completion.[[Type]] is throw, return ? completion.
                completion?;

                // 6. If innerResult.[[Type]] is throw, return ? innerResult.
                return Err(inner_result);
            }
        };

        // 5. If completion.[[Type]] is throw, return ? completion.
        let completion = completion?;

        // 6. If innerResult.[[Type]] is throw, return ? innerResult.
        let inner_result = inner_result?;

        if inner_result.is_object() {
            // 8. Return ? completion.
            Ok(completion)
        } else {
            // 7. If Type(innerResult.[[Value]]) is not Object, throw a TypeError exception.
            Err(JsNativeError::typ()
                .with_message("inner result was not an object")
                .into())
        }
    }

    /// `IteratorToList ( iteratorRecord )`
    ///
    /// More information:
    ///  - [ECMA reference][spec]
    ///
    ///  [spec]: https://tc39.es/ecma262/#sec-iteratortolist
    pub(crate) fn into_list(mut self, context: &mut Context) -> JsResult<Vec<JsValue>> {
        // 1. Let values be a new empty List.
        let mut values = Vec::new();

        // 2. Repeat,
        //     a. Let next be ? IteratorStepValue(iteratorRecord).
        while let Some(value) = self.step_value(context)? {
            // c. Append next to values.
            values.push(value);
        }

        //     b. If next is done, then
        //         i. Return values.
        Ok(values)
    }
}
