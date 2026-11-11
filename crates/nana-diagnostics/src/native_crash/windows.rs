//! The Windows x64 handlers and their writer thread.

use std::cell::UnsafeCell;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Once, TryLockError};
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, LocalFree};
use windows_sys::Win32::System::Diagnostics::Debug::{
    CONTEXT, EXCEPTION_POINTERS, RtlCaptureContext, SetUnhandledExceptionFilter,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, GetCurrentThreadId, GetCurrentThreadStackLimits, GetThreadDescription, INFINITE,
    OpenThread, SetEvent, THREAD_QUERY_LIMITED_INFORMATION, WaitForSingleObject,
};

use crate::crash::NATIVE_CRASH_REASON;
use crate::framework::diagnostics::NATIVE_CRASH;
use crate::record::{FaultRecord, Field, Record, pack};
use crate::runtime::{SYNC_SNAPSHOT_WAIT, global};
use crate::stall::windows::{MAX_FRAMES, describe, unwind};

const EXCEPTION_CONTINUE_SEARCH: i32 = 0;
/// The code `diagnostics.native_crash` carries for C `abort()`.
const STATUS_FATAL_APP_EXIT: u32 = 0x4000_0015;
const SIGABRT: i32 = 22;
const SIG_ERR: usize = usize::MAX;
/// How long a crashed thread waits for the writer before it lets the
/// process die anyway.
const WRITER_WAIT: u32 = 30_000;

unsafe extern "C" {
    /// The C runtime's `signal`; std links the UCRT on MSVC.
    fn signal(signal: i32, handler: usize) -> usize;
}

/// The crashed thread's registers. Written once, by the thread that won
/// `CRASHED_THREAD`, before it wakes the writer; read only by the writer.
#[repr(C, align(16))]
struct ContextSlot(UnsafeCell<CONTEXT>);

// SAFETY: access is serialized by `CRASHED_THREAD` and the start event.
unsafe impl Sync for ContextSlot {}

// SAFETY: an all-zero CONTEXT is a valid value.
static CONTEXT_SLOT: ContextSlot = ContextSlot(UnsafeCell::new(unsafe { std::mem::zeroed() }));
/// OS id of the thread being reported; 0 until a crash.
static CRASHED_THREAD: AtomicU32 = AtomicU32::new(0);
static WRITER_THREAD: AtomicU32 = AtomicU32::new(0);
/// Auto-reset: wakes the writer. Manual-reset: the report is done (or given
/// up), for every waiting thread.
static START: AtomicUsize = AtomicUsize::new(0);
static DONE: AtomicUsize = AtomicUsize::new(0);
static CODE: AtomicU32 = AtomicU32::new(0);
static ADDRESS: AtomicU64 = AtomicU64::new(0);
/// `ExceptionInformation[0..2]` and how many of them are set.
static PARAMETERS: [AtomicU64; 2] = [AtomicU64::new(0), AtomicU64::new(0)];
static PARAMETER_COUNT: AtomicU32 = AtomicU32::new(0);
static STACK_HIGH: AtomicU64 = AtomicU64::new(0);
static PREVIOUS_FILTER: AtomicUsize = AtomicUsize::new(0);
static PREVIOUS_ABORT: AtomicUsize = AtomicUsize::new(0);

/// Start the writer thread, then chain the unhandled-exception filter and
/// the `SIGABRT` handler in front of whatever was there. Once per process.
pub(crate) fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // SAFETY: plain Win32 event creation.
        let (start, done) = unsafe {
            (
                CreateEventW(std::ptr::null(), 0, 0, std::ptr::null()),
                CreateEventW(std::ptr::null(), 1, 0, std::ptr::null()),
            )
        };
        if start.is_null() || done.is_null() {
            return;
        }
        let (start, done) = (start as usize, done as usize);
        let (ready, wait) = std::sync::mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("nana-crash-writer".into())
            .spawn(move || {
                // SAFETY: plain Win32 call.
                WRITER_THREAD.store(unsafe { GetCurrentThreadId() }, Ordering::Release);
                let _ = ready.send(());
                writer(start, done);
            });
        if spawned.is_err() || wait.recv().is_err() {
            return;
        }
        START.store(start, Ordering::Release);
        DONE.store(done, Ordering::Release);
        // SAFETY: the handlers have the signatures Windows and the CRT call.
        unsafe {
            let previous = SetUnhandledExceptionFilter(Some(unhandled_exception));
            PREVIOUS_FILTER.store(previous.map_or(0, |f| f as usize), Ordering::Release);
            let previous = signal(SIGABRT, on_abort as *const () as usize);
            if previous != SIG_ERR {
                PREVIOUS_ABORT.store(previous, Ordering::Release);
            }
        }
    });
}

unsafe extern "system" fn unhandled_exception(info: *const EXCEPTION_POINTERS) -> i32 {
    // SAFETY: Windows passes valid exception pointers.
    unsafe {
        if !info.is_null() && !(*info).ExceptionRecord.is_null() {
            let record = &*(*info).ExceptionRecord;
            let count = record.NumberParameters.min(2);
            let parameters = [
                record.ExceptionInformation[0] as u64,
                record.ExceptionInformation[1] as u64,
            ];
            capture(
                record.ExceptionCode as u32,
                record.ExceptionAddress as u64,
                parameters,
                count,
                (*info).ContextRecord,
            );
        }
        match PREVIOUS_FILTER.load(Ordering::Acquire) {
            0 => EXCEPTION_CONTINUE_SEARCH,
            previous => {
                let previous: unsafe extern "system" fn(*const EXCEPTION_POINTERS) -> i32 =
                    std::mem::transmute(previous);
                previous(info)
            }
        }
    }
}

extern "C" fn on_abort(signal: i32) {
    capture(STATUS_FATAL_APP_EXIT, 0, [0; 2], 0, std::ptr::null());
    let previous = PREVIOUS_ABORT.load(Ordering::Acquire);
    // SIG_DFL (0) and SIG_IGN (1) are not functions.
    if previous > 1 {
        // SAFETY: the CRT returned it as a handler for this signal.
        let previous: extern "C" fn(i32) = unsafe { std::mem::transmute(previous) };
        previous(signal);
    }
}

/// Runs on the crashed thread: no allocation, no lock, little stack.
#[inline(never)]
fn capture(code: u32, address: u64, parameters: [u64; 2], count: u32, context: *const CONTEXT) {
    let start = START.load(Ordering::Acquire);
    let done = DONE.load(Ordering::Acquire);
    // SAFETY: plain Win32 calls; the slot is ours once the exchange below
    // succeeds, and `context` (when not null) is the OS's exception context.
    unsafe {
        let thread = GetCurrentThreadId();
        // The writer itself crashed: nobody is left to write.
        if start == 0 || thread == WRITER_THREAD.load(Ordering::Acquire) {
            return;
        }
        if let Err(owner) =
            CRASHED_THREAD.compare_exchange(0, thread, Ordering::AcqRel, Ordering::Acquire)
        {
            // Another thread's crash is being written; let it finish before
            // this one takes the process down. A crash inside our own report
            // returns at once.
            if owner != thread {
                WaitForSingleObject(done as HANDLE, WRITER_WAIT);
            }
            return;
        }
        let slot = CONTEXT_SLOT.0.get();
        if context.is_null() {
            RtlCaptureContext(slot);
        } else {
            std::ptr::copy_nonoverlapping(context, slot, 1);
        }
        let (mut low, mut high) = (0usize, 0usize);
        GetCurrentThreadStackLimits(&mut low, &mut high);
        STACK_HIGH.store(high as u64, Ordering::Relaxed);
        CODE.store(code, Ordering::Relaxed);
        ADDRESS.store(address, Ordering::Relaxed);
        PARAMETERS[0].store(parameters[0], Ordering::Relaxed);
        PARAMETERS[1].store(parameters[1], Ordering::Relaxed);
        PARAMETER_COUNT.store(count, Ordering::Release);
        SetEvent(start as HANDLE);
        WaitForSingleObject(done as HANDLE, WRITER_WAIT);
    }
}

fn writer(start: usize, done: usize) {
    // SAFETY: the events live for the process.
    unsafe { WaitForSingleObject(start as HANDLE, INFINITE) };
    report();
    // SAFETY: as above.
    unsafe { SetEvent(done as HANDLE) };
}

fn report() {
    let thread = CRASHED_THREAD.load(Ordering::Acquire);
    let code = CODE.load(Ordering::Relaxed);
    let address = ADDRESS.load(Ordering::Relaxed);
    let count = PARAMETER_COUNT.load(Ordering::Acquire);
    let parameters = [
        PARAMETERS[0].load(Ordering::Relaxed),
        PARAMETERS[1].load(Ordering::Relaxed),
    ];
    let Some(diagnostics) = global() else {
        return;
    };
    let shared = &diagnostics.shared;

    // First the breadcrumb: no allocation, synced, survives whatever the
    // rest of this runs into.
    for _ in 0..100 {
        match shared.marker.try_lock() {
            Ok(mut marker) => {
                if let Some(marker) = marker.as_mut() {
                    marker.record_exception(
                        u64::from(code),
                        address,
                        crate::session::unix_now_ns(),
                    );
                }
                break;
            }
            Err(TryLockError::Poisoned(poisoned)) => {
                if let Some(marker) = poisoned.into_inner().as_mut() {
                    marker.record_exception(
                        u64::from(code),
                        address,
                        crate::session::unix_now_ns(),
                    );
                }
                break;
            }
            Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(5)),
        }
    }

    // The crashed thread's stack, from the registers it crashed with. It is
    // blocked in `capture`, so everything above its stack pointer holds
    // still.
    // SAFETY: written before the start event was set, never again.
    let mut context = unsafe { std::ptr::read(CONTEXT_SLOT.0.get()) };
    let high = STACK_HIGH.load(Ordering::Relaxed);
    let low = context.Rsp;
    let mut frames = [0u64; MAX_FRAMES];
    let mut found = 0;
    if low < high && low.is_multiple_of(8) {
        // A call through a null pointer faults with nothing at rip: the
        // caller's return address is on top of the stack.
        if context.Rip == 0 && low + 8 <= high {
            // SAFETY: inside the crashed thread's stack, above its pointer.
            context.Rip = unsafe { *(low as *const u64) };
            context.Rsp += 8;
            frames[0] = 0;
            found = 1;
        }
        found += unwind(&mut context, low, high, &mut frames[found..]);
    }

    let mut text = String::with_capacity(4096);
    if code == STATUS_FATAL_APP_EXIT && address == 0 {
        text.push_str("abort() called (SIGABRT)");
    } else {
        let _ = write!(
            text,
            "unhandled exception 0x{code:08X} ({}) at 0x{address:x}",
            name(code)
        );
        if (code == 0xC000_0005 || code == 0xC000_0006) && count >= 2 {
            let access = match parameters[0] {
                0 => "reading",
                1 => "writing",
                8 => "executing",
                _ => "accessing",
            };
            let _ = write!(text, ", {access} 0x{:x}", parameters[1]);
        }
    }
    let _ = write!(text, "\nthread '{}' ({thread})", thread_name(thread));
    text.push_str("\nstack:\n");
    if found == 0 {
        text.push_str("  <unavailable>\n");
    } else {
        describe(&frames[..found], &mut text);
    }

    let event = &NATIVE_CRASH;
    let (len, values) = pack(
        event,
        &[
            Field::new("code", u64::from(code)),
            Field::new("address", address),
            Field::new("os_thread", u64::from(thread)),
        ],
    );
    let fault = FaultRecord {
        record: Record {
            ts_ns: shared.now_ns(),
            event,
            thread: 0,
            len,
            values,
        },
        message: Some(text.into_boxed_str()),
    };
    let _ =
        crate::worker::snapshot_now(shared, NATIVE_CRASH_REASON, Some(fault), SYNC_SNAPSHOT_WAIT);
}

fn name(code: u32) -> &'static str {
    match code {
        0xC000_0005 => "access violation",
        0xC000_0006 => "in-page error",
        0xC000_001D => "illegal instruction",
        0xC000_0025 => "noncontinuable exception",
        0xC000_008C => "array bounds exceeded",
        0xC000_008E => "float divide by zero",
        0xC000_0090 => "float invalid operation",
        0xC000_0094 => "integer divide by zero",
        0xC000_0095 => "integer overflow",
        0xC000_0096 => "privileged instruction",
        0xC000_00FD => "stack overflow",
        0xC000_0374 => "heap corruption",
        0xC000_0409 => "stack buffer overrun",
        0x8000_0003 => "breakpoint",
        0xE06D_7363 => "C++ exception",
        _ => "exception",
    }
}

/// The thread's description (std sets it for named threads), if it has one.
fn thread_name(thread: u32) -> String {
    let mut name = String::from("<unnamed>");
    // SAFETY: plain Win32 calls; the description is freed with LocalFree as
    // documented.
    unsafe {
        let handle = OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, thread);
        if handle.is_null() {
            return name;
        }
        let mut description = std::ptr::null_mut();
        if GetThreadDescription(handle, &mut description) >= 0 && !description.is_null() {
            let len = (0..).take_while(|&i| *description.add(i) != 0).count();
            if len > 0 {
                name = String::from_utf16_lossy(std::slice::from_raw_parts(description, len));
            }
            LocalFree(description.cast());
        }
        CloseHandle(handle);
    }
    name
}
