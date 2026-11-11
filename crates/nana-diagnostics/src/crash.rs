//! Best-effort crash capture, and the query that finds the last crash.
//!
//! - Rust panics, including under `panic = "abort"` (the hook runs before
//!   the abort): a `diagnostics.panic` fault and a `panic` snapshot.
//! - Native crashes on Windows x64 (access violations, stack overflows,
//!   other unhandled SEH exceptions, C `abort()`): see `native_crash`; a
//!   `diagnostics.native_crash` fault and a `native-crash` snapshot.
//! - Everything nothing in-process can see (a killed process, power loss,
//!   `std::process::abort`'s fail-fast, a crash the writer could not finish):
//!   the run marker (`marker`), reported by the next session as
//!   `diagnostics.unclean_exit` with an `unclean-exit` snapshot.
//!
//! [`last_crash`] finds the newest of those snapshots.

use std::cell::Cell;
use std::fs;
use std::panic::PanicHookInfo;
use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::{Duration, SystemTime};

use crate::files::{is_session_file, sanitize};

use crate::framework;
use crate::record::{FaultRecord, Record};
use crate::runtime::{Diagnostics, global};

pub(crate) const PANIC_REASON: &str = "panic";
pub(crate) const NATIVE_CRASH_REASON: &str = "native-crash";
pub(crate) const UNCLEAN_EXIT_REASON: &str = "unclean-exit";

/// How long a panicking thread waits for the worker's state lock.
const PANIC_WAIT: Duration = Duration::from_millis(500);

static HOOK: Once = Once::new();

thread_local! {
    static IN_HOOK: Cell<bool> = const { Cell::new(false) };
}

/// Chain a hook in front of the current panic hook: record a Fatal fault,
/// write a flight-recorder snapshot to the crash directory, then call the
/// previous hook. Installs once per process; a no-op while no global
/// instance is installed.
pub fn install_panic_hook() {
    HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            capture_panic(info);
            previous(info);
        }));
    });
}

fn capture_panic(info: &PanicHookInfo<'_>) {
    // A panic inside our own capture must not recurse.
    if IN_HOOK.with(|flag| flag.replace(true)) {
        return;
    }
    if let Some(diagnostics) = global() {
        let shared = &diagnostics.shared;
        let thread = std::thread::current();
        let name = thread.name().unwrap_or("<unnamed>");
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string payload>");
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let message = format!("thread '{name}' panicked at {location}: {payload}");
        let fault = FaultRecord {
            record: Record {
                ts_ns: shared.now_ns(),
                event: &framework::diagnostics::PANIC,
                thread: 0,
                len: 0,
                values: [0; crate::schema::MAX_FIELDS],
            },
            message: Some(message.into_boxed_str()),
        };
        let _ = crate::worker::snapshot_now(shared, PANIC_REASON, Some(fault), PANIC_WAIT);
    }
    IN_HOOK.with(|flag| flag.set(false));
}

/// How a session ended badly, as its crash-directory snapshot tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrashKind {
    /// A Rust panic (`-panic` snapshot).
    Panic,
    /// An unhandled native exception or C `abort()` (`-native-crash`).
    NativeCrash,
    /// A session that disappeared without shutting down, found by the next
    /// one (`-unclean-exit`, written by that next session).
    UncleanExit,
}

/// The newest crash snapshot of an app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastCrash {
    pub kind: CrashKind,
    /// The snapshot's modification time: when it was written.
    pub time: SystemTime,
    pub path: PathBuf,
}

fn kind_of(reason: &str) -> Option<CrashKind> {
    // `write_new_file` appends `-{n}` when the name is taken.
    let reason = match reason.rsplit_once('-') {
        Some((head, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => head,
        _ => reason,
    };
    match reason {
        PANIC_REASON => Some(CrashKind::Panic),
        NATIVE_CRASH_REASON => Some(CrashKind::NativeCrash),
        UNCLEAN_EXIT_REASON => Some(CrashKind::UncleanExit),
        _ => None,
    }
}

/// The crash kind of `name` if it is a crash snapshot of the session `stem`.
pub(crate) fn snapshot_kind(name: &str, stem: &str) -> Option<CrashKind> {
    kind_of(
        name.strip_prefix(stem)?
            .strip_prefix('-')?
            .strip_suffix(".nlog")?,
    )
}

/// The crash kind of `name` if it is a crash snapshot of any session of the
/// (sanitized) app `app`: `{app}-{stamp}-{pid}-{reason}[-{n}].nlog`.
fn app_snapshot_kind(name: &str, app: &str) -> Option<CrashKind> {
    if !is_session_file(name, app) {
        return None;
    }
    // `{app}-` and the 16-byte stamp with its `-` were checked above.
    let rest = &name[app.len() + 1 + 17..];
    let rest = rest.trim_start_matches(|c: char| c.is_ascii_digit());
    kind_of(rest.strip_prefix('-')?.strip_suffix(".nlog")?)
}

/// The newest panic, native-crash or unclean-exit snapshot of `app_id` in the
/// crash directory `dir`.
pub fn last_crash_in(dir: &Path, app_id: &str) -> Option<LastCrash> {
    let app = sanitize(app_id);
    fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let kind = app_snapshot_kind(name.to_str()?, &app)?;
            let time = entry.metadata().ok()?.modified().ok()?;
            Some(LastCrash {
                kind,
                time,
                path: entry.path(),
            })
        })
        .max_by_key(|crash| crash.time)
}

/// The newest crash of the global instance's app (see
/// [`Diagnostics::last_crash`]).
pub fn last_crash() -> Option<LastCrash> {
    global()?.last_crash()
}

impl Diagnostics {
    /// The newest crash of this app in the crash directory: a panic, a
    /// native crash, or a session that ended without shutting down (whose
    /// `unclean-exit` snapshot [`Diagnostics::start`] writes before it
    /// returns). [`Diagnostics::unclean_exits`] has the details of the last.
    pub fn last_crash(&self) -> Option<LastCrash> {
        last_crash_in(self.paths().crash.as_deref()?, &self.metadata().app_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_snapshot_names_are_recognised_by_reason() {
        let app = "dev.nana";
        let stem = "dev.nana-20260922T010203Z-42";
        for (name, kind) in [
            (
                "dev.nana-20260922T010203Z-42-panic.nlog",
                Some(CrashKind::Panic),
            ),
            (
                "dev.nana-20260922T010203Z-42-panic-2.nlog",
                Some(CrashKind::Panic),
            ),
            (
                "dev.nana-20260922T010203Z-42-native-crash.nlog",
                Some(CrashKind::NativeCrash),
            ),
            (
                "dev.nana-20260922T010203Z-42-unclean-exit.nlog",
                Some(CrashKind::UncleanExit),
            ),
            ("dev.nana-20260922T010203Z-42-stall.nlog", None),
            ("dev.nana-20260922T010203Z-42-device-lost.nlog", None),
            ("dev.nana-20260922T010203Z-42.nlog", None),
            ("dev.nana-20260922T010203Z-42.3.nlog", None),
        ] {
            assert_eq!(app_snapshot_kind(name, app), kind, "{name}");
            assert_eq!(snapshot_kind(name, stem), kind, "{name}");
        }
        assert_eq!(
            app_snapshot_kind("dev.nana.beta-20260922T010203Z-42-panic.nlog", app),
            None
        );
    }
}
