//! Recursions over the tree that must not run out of stack, whatever the
//! tree's depth and whatever the thread they run on.
//!
//! Only the passes that recurse by nature go through here: measure (sizes
//! come up from the children a constraint goes down to) and building a view
//! (an element builds its children inside its own scope). Every other walk
//! of the tree keeps its open levels on the heap, and placement descends a
//! bounded number of levels before handing the rest to its scheduler (see
//! `layout_engine::recursion`), so neither needs this.
//!
//! A recursion reads how much stack it has left at each level: below
//! [`RED_ZONE`] it carries on in a fresh [`SEGMENT`] (`stacker::grow`).
//! `stacker` reads the limit on desktop targets and OpenHarmony (a Linux
//! target); on iOS and Android, where it cannot, [`remaining`] asks the
//! thread itself. So the guard holds on every platform the framework
//! supports; a target where neither can tell would run each level where it
//! is.

/// Stack left below which a level moves to a new segment: room for one level
/// and the non-recursive work it calls before the next level checks again.
pub(crate) const RED_ZONE: usize = 256 << 10;
/// What a level that moves gets.
pub(crate) const SEGMENT: usize = 4 << 20;

/// Stack left on this thread, in bytes; `None` where the platform cannot
/// tell.
pub(crate) fn remaining() -> Option<usize> {
    // Inside a segment `stacker` knows its bounds on every platform; on a
    // thread's own stack it knows them on desktop targets only.
    stacker::remaining_stack().or_else(native::remaining)
}

/// Runs one level of a recursion: here, or on a new segment when this stack
/// is nearly used up.
pub(crate) fn level<T>(run: impl FnOnce() -> T) -> T {
    if remaining().is_some_and(|left| left < RED_ZONE) {
        stacker::grow(SEGMENT, run)
    } else {
        run()
    }
}

/// Runs `run` in a frame of its own.
///
/// Unoptimized code keeps a stack slot for every local of a function, so a
/// `match` whose arms each build something large holds all of their locals
/// at once, whichever arm runs: a debug frame of 100 KiB on the per-frame
/// path. An arm moved in here takes its locals only when it runs. An
/// optimized build inlines it, so it costs nothing there.
#[inline(always)]
pub(crate) fn own_frame<T>(run: impl FnOnce() -> T) -> T {
    run()
}

#[cfg(any(
    target_os = "android",
    all(target_vendor = "apple", not(target_os = "macos"))
))]
mod native {
    use std::cell::Cell;

    thread_local! {
        /// The lowest address of this thread's stack, read once per thread.
        static LIMIT: Cell<Option<Option<usize>>> = const { Cell::new(None) };
    }

    pub(super) fn remaining() -> Option<usize> {
        let marker = 0u8;
        let here = std::ptr::addr_of!(marker) as usize;
        let limit = LIMIT.with(|cell| {
            let limit = cell.get().unwrap_or_else(lowest_address);
            cell.set(Some(limit));
            limit
        })?;
        Some(here.saturating_sub(limit))
    }

    #[cfg(target_vendor = "apple")]
    fn lowest_address() -> Option<usize> {
        // SAFETY: both read the calling thread's own attributes.
        let (top, size) = unsafe {
            let thread = libc::pthread_self();
            (
                libc::pthread_get_stackaddr_np(thread) as usize,
                libc::pthread_get_stacksize_np(thread),
            )
        };
        top.checked_sub(size)
    }

    #[cfg(target_os = "android")]
    fn lowest_address() -> Option<usize> {
        // SAFETY: `attr` is initialised by `pthread_getattr_np` before it is
        // read, and destroyed once.
        unsafe {
            let mut attr = std::mem::MaybeUninit::<libc::pthread_attr_t>::uninit();
            if libc::pthread_getattr_np(libc::pthread_self(), attr.as_mut_ptr()) != 0 {
                return None;
            }
            let mut base = std::ptr::null_mut();
            let mut size = 0;
            let read = libc::pthread_attr_getstack(attr.as_ptr(), &mut base, &mut size);
            libc::pthread_attr_destroy(attr.as_mut_ptr());
            (read == 0).then_some(base as usize)
        }
    }
}

#[cfg(not(any(
    target_os = "android",
    all(target_vendor = "apple", not(target_os = "macos"))
)))]
mod native {
    pub(super) fn remaining() -> Option<usize> {
        None
    }
}
