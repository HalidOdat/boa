//! Implements object shapes.

pub(crate) mod property_table;
mod root_shape;
pub(crate) mod shared_shape;
pub(crate) mod slot;

pub use root_shape::RootShape;
pub use shared_shape::SharedShape;

use self::slot::Slot;

/// Action to be performed after a property attribute change
//
// Example: of { get/set x() { ... }, y: ... } into { x: ..., y: ... }
//
//                 0       1       2
//    Storage: | get x | set x |   y   |
//
// We delete at position of x which is index 0 (it spans two elements) + 1:
//
//                 0      1
//    Storage: |   x  |   y   |
pub(crate) enum ChangeTransitionAction {
    /// Do nothing to storage.
    Nothing,

    /// Remove element at (index + 1) from storage.
    Remove,

    /// Insert element at (index + 1) into storage.
    Insert,
}

/// The result of a change property attribute transition.
pub(crate) struct ChangeTransition {
    /// The shape after transition.
    pub(crate) shape: SharedShape,

    /// The needed action to be performed after transition to the object storage.
    pub(crate) action: ChangeTransitionAction,
}
