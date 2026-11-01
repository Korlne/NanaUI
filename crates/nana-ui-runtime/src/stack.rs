//! Recursions over the tree that must not run out of stack, whatever the
//! tree's depth and whatever the thread they run on.
//!
//! A recursion reads how much stack it has left at each level: below
//! [`RED_ZONE`] it carries on in a fresh [`SEGMENT`] (`stacker::grow`). Where
//! the platform cannot tell what is left (`stacker::remaining_stack` is
//! `None`: Android, iOS, wasm), the level runs where it is.

/// Stack left below which a level moves to a new segment: room for one level
/// and the non-recursive work it calls before the next level checks again.
pub(crate) const RED_ZONE: usize = 256 << 10;
/// What a level that moves gets.
pub(crate) const SEGMENT: usize = 4 << 20;

/// Runs one level of a recursion: here, or on a new segment when this stack
/// is nearly used up.
pub(crate) fn level<T>(run: impl FnOnce() -> T) -> T {
    if stacker::remaining_stack().is_some_and(|left| left < RED_ZONE) {
        stacker::grow(SEGMENT, run)
    } else {
        run()
    }
}
