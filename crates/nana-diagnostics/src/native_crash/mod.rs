//! Native crash capture: unhandled SEH exceptions and C `abort()` on
//! Windows x64. A no-op elsewhere; the run marker still catches those
//! sessions on the next start.
//!
//! The crashed thread does as little as possible: it copies its registers
//! and stack bounds into static slots, wakes a writer thread created at
//! install time, and waits for it. That keeps the work off a thread that may
//! have no stack left (an overflow leaves the few KiB of
//! `SetThreadStackGuarantee` that std reserves on its threads) and needs no
//! heap there. The writer first leaves an allocation-free breadcrumb (code
//! and address, synced) in the run marker, then walks the crashed thread's
//! stack (it is blocked, so its frames hold still), symbolizes it, and writes
//! a `diagnostics.native_crash` fault and a `native-crash` snapshot like the
//! panic hook does. If the crash left the heap or loader lock held, those
//! later steps can block; the crashed thread stops waiting after
//! `WRITER_WAIT` and lets the process die with the breadcrumb on disk.
//!
//! Not catchable in-process: `std::process::abort`, `handle_alloc_error`'s
//! default and other `__fastfail`s go straight to the kernel. The run marker
//! reports those sessions as unclean exits.

#[cfg(all(windows, target_arch = "x86_64"))]
mod windows;

#[cfg(all(windows, target_arch = "x86_64"))]
pub(crate) use windows::install;

/// Nothing to install on this platform.
#[cfg(not(all(windows, target_arch = "x86_64")))]
pub(crate) fn install() {}
