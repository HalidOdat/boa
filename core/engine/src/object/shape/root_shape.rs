use boa_macros::{Finalize, Trace};

use super::Shape;

/// This is a wrapper around [`Shape`] that ensures it's root shape.
///
/// Represent the root shape that [`Shape`] transitions start from.
#[derive(Debug, Clone, Trace, Finalize)]
pub struct RootShape {
    shape: Shape,
}

impl Default for RootShape {
    #[inline]
    fn default() -> Self {
        Self {
            shape: Shape::root(),
        }
    }
}

impl RootShape {
    /// Gets the inner [`Shape`].
    #[must_use]
    pub const fn shape(&self) -> &Shape {
        &self.shape
    }
}
