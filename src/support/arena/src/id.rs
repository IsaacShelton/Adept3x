use core::fmt::Debug;

/// A trait for index types used in arenas.
///
/// An [`Id`] represents both the internal index in an arena and a type-level distinction
/// (for example, when using multiple arenas with the same underlying numeric index type).
pub trait Id: Copy + Ord + Debug {
    /// The maximum value (as a usize) this id type can represent.
    const MAX: usize;

    /// Converts a `usize` value to this id type.
    ///
    /// The input `idx` (should / is guaranteed to) be less than `Self::MAX`.
    fn from_usize(idx: usize) -> Self;

    /// Converts this id type into a `usize`.
    ///
    /// The returned value (should / is guaranteed to) be less than `Self::MAX`.
    fn into_usize(self) -> usize;

    /// Gets the successor of this id
    fn successor(self) -> Self;
}
