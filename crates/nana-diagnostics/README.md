# nana-diagnostics

English · [简体中文](README.zh-CN.md)

Low-overhead structured diagnostics: events, metrics, faults, an in-memory flight recorder, and the `.nlog` file format.

Call sites use `event!`, `metric!`, `fault!`, and `span!` with static descriptors. Events are written into a thread-local ring buffer and dropped when it is full; a background worker writes them to `.nlog` in batches while keeping roughly the last minute of records in memory, and a snapshot is written synchronously on panic, on a native crash (Windows x64: unhandled SEH exceptions and C `abort()`), and on GPU device loss. A run marker in the crash directory lets the next session report a process that was killed or died without any record; `last_crash()` finds the newest of these. Producer threads do no file I/O, locking, or formatting, and with no instance installed each call site is a single atomic read. NanaUI's own descriptors are in the `framework` module; domains `0x0000..=0x00FF` are reserved for the framework, and applications start at `Domain::APPLICATION_MIN`.

This crate depends on no other NanaUI crate. Log and crash directories are resolved by `ApplicationPaths` in [`nana-ui-platform`](../nana-ui-platform) and passed in; redaction and reporting are up to the application.

## Example

```rust
use nana_diagnostics::{
    Domain, EventDescriptor, FieldDescriptor, HistogramCells, Metric, Severity, event, metric,
};

const LIVE: Domain = Domain(0x0100);
static MODEL_LOADED: EventDescriptor = EventDescriptor::new(
    LIVE, 1, "live.model_loaded", Severity::Info, &[FieldDescriptor::u64("bytes")],
);
static TRACK_CELLS: HistogramCells = HistogramCells::new();
static TRACK_NS: Metric = Metric::histogram(LIVE, 1, "live.track", "ns", &TRACK_CELLS);

event!(MODEL_LOADED, bytes = 1024u64);
metric!(TRACK_NS, std::time::Duration::from_micros(250));
```

To view a `.nlog`, use the bundled `nana-nlog`:

```bash
cargo run -p nana-diagnostics --bin nana-nlog -- text <file.nlog>
```

## Tests

```bash
cargo test -p nana-diagnostics
```

Hot-path benchmarks and the remaining checks are in [`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).

See also [Diagnostics and application paths](../../docs/en/reference/diagnostics.md).
