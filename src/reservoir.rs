/// Utilities for the reservoir data structure.
use errors::*;

/// A data structure that can be drained of its items until empty, then automatically refilled
/// with a provided `fill` function.
pub trait Reservoir<V> {
    /// Returns an `Ok` containing the next item in the reservoir or `Err` if it could
    /// not be provided.
    ///
    /// # Arguments
    /// * `fill_fn` - A function used to refill the reservoir with items. This may be called
    ///               zero or more times by the implemenation.`
    fn next_or_fill<F>(self, fill_fn: F) -> Result<V>
    where
        F: Fn() -> Result<Vec<V>>;
}
