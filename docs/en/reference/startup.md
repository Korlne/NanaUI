# Two-phase startup and the native splash contract

This page specifies NanaUI's two-phase startup (Issue #225): without introducing a second UI engine, it decouples **native first-screen feedback (Early Splash)** from **full UI readiness (UiReady / visual takeover)**, so the user no longer waits on a blank window while the application initializes and shaders compile.

## The two phases and the core boundary

1. **Phase one: Early Splash (native startup window)**. While the GPU device, pipelines, and application business state are not yet ready, the host first shows a separate, non-activating, small native splash window. The logo can be compiled into the binary or read once from an `early-splash` resource pack. Animations such as pulsing or rotation are driven directly by the operating system compositor (macOS Render Server / Windows DWM); this process runs zero timers and zero per-frame callbacks (`SplashWork::animation_submissions` is always `1`).
2. **Phase two: `UiReady` and visual takeover**. Once the window, Surface, GPU device, `SceneWgpuPainter`, and text system are all ready and an ordinary document can be mounted, laid out, and painted, the host enters `UiReady` and calls `RuntimeProgram::initialize`. The application mounts a loading page or its main UI there and declares a takeover policy. **The host removes the native splash only after it has confirmed that the first frame of the takeover content was successfully submitted to the system compositor.**

::: tip Capability handoff vs. visual handoff
`UiReady` only means "UI rendering capability is ready". It does not mean that long-running business loading has finished. If `early_splash` is not configured, the host creates no extra layer or native window, and behavior is exactly the same as traditional single-phase startup.
:::

## Quick setup

```rust
static LOGO: &[u8] = include_bytes!("../assets/logo.png");

NanaApplication::builder(identity)
    .early_splash(
        SplashSpec::new(SplashLogo::png(LOGO))
            .with_logo_size(128.0, 128.0)
            .with_animation(SplashAnimation::Pulse),
    )
    .run::<RuntimeApplication<App>>(WindowDescriptor::new("App"))
```

Hosts that do not go through `NanaApplication::builder` can use `nana_ui::with_startup(StartupOptions::default().with_splash(spec), || run_runtime::<P>(settings))`. For a complete runnable demo and acceptance probe, see `crates/nana-ui/examples/startup-splash.rs` (pass `--probe` to print a structured JSON timing record after the handoff completes and to check the contract).

## Phase one: Early Splash configuration and platform implementation

`SplashSpec` has the following four settings. This phase provides no text layout, progress bar, scripting, or custom shaders; to show the product name, bake it into the logo image.

| Field | Description |
| --- | --- |
| `logo` | `SplashLogo::png(&'static [u8])` (an embedded static PNG) or `SplashLogo::packaged("nana://res/…")` (read from an `early-splash` resource pack). Encoded size limit `1 MiB`, longest-side limit `1024 px`, decoded pixel limit `4 MiB` (the PNG header is validated before startup; anything over a limit is rejected without decoding). |
| `logo_size` | The logical pixel size of the separate splash window and the logo content box (the logo is scaled proportionally and centered; the separate splash window does not appear in the taskbar, does not steal focus, and only follows the main window's center point and DPI). |
| `background` | `System` (the default theme palette's background color for the current system light/dark mode), `Color(..)`, or `Transparent` (draw only the logo). If the application's own theme does not follow the system light/dark mode, specify `Color(..)` explicitly to avoid a background color jump at handoff. |
| `animation` | `None`, `FadeIn` (default; fade in once and hold), `Pulse` (breathing pulse), `Rotate` (continuous rotation). |

### Cross-platform native implementation and handoff

| Platform | Native splash implementation | System compositor animation | Flicker-free first-frame handoff |
| --- | --- | --- | --- |
| **macOS** | A separate `NSWindow` with no title bar, a transparent background, and mouse events ignored, plus a `CALayer` | `CABasicAnimation` (driven by the Render Server) | During the splash the main window is kept at `alphaValue = 0` (it can lay out but is invisible and receives no pointer input); after the takeover target frame is `present`ed, the main window's `alphaValue` is restored to `1` and the splash is removed with `orderOut` |
| **Windows** | A DirectComposition visual tree on a separate non-activating `WS_POPUP` window (a short-lived D3D11 device uploads the bitmap once and is released right after handoff) | `IDCompositionAnimation` (driven by DWM) | During the splash the main window is hidden with `DWMWA_CLOAK`; after the takeover target frame is `present`ed and `on_submitted_work_done` has completed: uncloak → `DwmFlush()` → remove the splash visual and release D3D11/DComp |
| **Linux and others** | No native splash is shown; `Skipped(PlatformUnsupported)` is reported directly | — | `UiReady` and the takeover ticket contract work as usual; application code needs no platform branches |

The applied outcome is recorded in `StartupStatus::splash` (`SplashOutcome`); the requested value is never treated as the result: `Shown { animation: Applied(..) | Static { requested, reason } }` (when the system has "reduce motion" enabled or the compositor rejects the animation, it automatically degrades to a static logo and records `ReducedMotion` / `NativeAnimationFailed`), `Skipped(..)` (not configured, hidden startup with `visible = false`, embedded host, or unsupported platform), or `Failed(Logo(..) | Package(..) | Native(..))` (a corrupt or oversized logo does not block application startup; only the splash is skipped).

### Reading the logo from an `early-splash` resource pack

With the `nana-ui` `packaged-resources` feature enabled and `resource_packs` configured, `SplashLogo::packaged("nana://res/splash/logo.png")` reads the logo from a resource pack in the `early-splash` category (the time is recorded in `StartupWork::splash_logo_read`; measured locally, a 71 KB logo takes about `0.25 ms`):

- The path is routed to exactly one pack by longest-prefix match in the signature-verified `package.json`. That pack must belong to the unencrypted `early-splash` category (otherwise opening is refused with `NotEarlySplash`, and `KeyProvider` is never called).
- After reading the 4 KiB header, the reader checks the total file size (over `4 MiB` reports `EarlySplashTooLarge`), checks the TOC to confirm the entry is at most `1 MiB`, then verifies hashes block by block and checks the PNG header (during development, when running from `target/`, it automatically falls back to the file of the same name under `loose_root`).

## Phase two: `UiReady` and visual takeover

`UiReady` is reached when `RuntimeProgram::initialize` (or `ApplicationState::initialize` + `build`) is called. At that point `initialize` only needs to mount the first-screen view (a loading page or the main UI), dispatch long-running business work to background threads, and return immediately.

`RuntimeProgram::startup_takeover()` decides when the native splash is removed:

- **`Immediate` (default)**: the main window document built by `initialize` is the takeover content. (When a splash is configured, the initial messages returned by `initialize` are processed in batches of `2 ms / 64 messages` before takeover is requested, so the frame that removes the logo already includes the effects of the initial messages.)
- **`Deferred`**: keep showing the splash (or the loading state) until the application explicitly calls `context.startup().take_over(ticket)`:

```rust
let startup = context.startup();
let ticket = startup.status().ticket.expect("a ticket is assigned after UiReady");
startup.take_over(ticket)?;          // callable from any thread
startup.cancel_takeover(ticket)?;    // withdraw the takeover request and invalidate the old ticket
```

- **Generational tickets and target-frame confirmation**: calling `cancel_takeover` invalidates the old `ticket` (if the takeover frame was already presented, it returns `AlreadyHandedOff`; a late background task calling with an old ticket receives `StaleTicket`). `take_over` records the main window's current `flush` sequence number. **Only a main window frame that completes `flush` after that point and is actually `present`ed successfully completes the handoff** (skipped frames, failed frames, and old frames from before a device rebuild do not count).

## Parallel startup threads and faster shader compilation

After the standalone host (`run_runtime`) creates the window and Surface on the window thread, it immediately moves GPU adapter selection, the device request, and `SceneWgpuPainter` pipeline compilation onto the background `nana-startup-gpu` thread, while mounting the native splash on the main thread. **Splash display therefore fully overlaps GPU initialization.**

Inside the `nana-startup-gpu` thread:
1. If a composition backend was requested (`CompositionCapable`), it reuses the adapter list already enumerated by the DX12 composition probe that ran before window creation, skipping a second enumeration (time drops from about `0.74 s` to `0.08 s`); without a probe it enumerates once here. An adapter is picked from this list according to `StartupOptions::adapter`; see [GPU · Adapter selection](gpu.md#adapter-selection) for the rules.
2. The major `SceneWgpuPainter` pipeline families (quad, mesh/path, icon, text, HostTexture, backdrop) compile in parallel on scoped threads, so total build time is set by the slowest single family (quad) rather than the sum of all of them. The motion evaluation pipeline, used only for test readback, is not compiled on the product startup path.
3. On macOS, the only slow step, Dock icon resampling and PNG encoding, runs asynchronously on a separate `nana-startup-icons` thread and does not block `UiReady`.

### DX12 shader compiler

On Windows DX12, almost all of `SceneWgpuPainter`'s startup build time is spent compiling shaders. **DXC is about twice as fast as the system's built-in FXC** (measured on an RTX 5060 Ti: release-build painter compilation drops from FXC's `2.24–2.39 s` to DXC's `1.18–1.79 s`; debug builds drop from `4.73–7.77 s` to `1.61–1.98 s`):

- When an x64 `dxcompiler.dll` is present in the same directory as the executable (`.exe`) (DXC `v1.8.2502` or later is recommended, from [DirectXShaderCompiler releases](https://github.com/microsoft/DirectXShaderCompiler/releases) or Windows SDK `10.0.26100`; `dxil.dll` is not needed), the host loads it first by its full absolute path (unaffected by `PATH` or the current working directory).
- Without `dxcompiler.dll` it automatically falls back to the system's built-in FXC. You can also choose explicitly with the `WGPU_DX12_COMPILER` environment variable (`fxc` / `dxc` / `staticdxc` / `auto`).

### Precompiled DXIL

The shader stages of the quad, mesh/path and text families (the dual-source subpixel program included) are compiled to DXIL (shader model 6.0) inside the framework ahead of time and ship in the binary (`scene_paint::precompiled`). The host requests `PASSTHROUGH_SHADERS` where the DX12 adapter supports it, and those families then load the DXIL directly instead of going through naga → HLSL → DXC; the driver only lowers DXIL to its own code, which it caches across launches. Measured (RTX 5060 Ti, release, DXC): the quad family `1.26–1.30 s` → `1.4–1.6 ms`, mesh `199–248 ms` → `2 ms`, text `126–160 ms` → `2 ms`; the whole painter drops from `1.40–1.74 s` compiling all WGSL, and about `250 ms` with only quad precompiled, to `75–84 ms`, where the slowest families are now backdrop and dest (about `90–105 ms`).

- Only the framework's own DXIL, built into the binary: a passthrough module is neither parsed nor validated by naga, so DXIL never comes from a file, a skin or a plugin, and application and skin shaders stay WGSL.
- Each family records which program (a hash of the comment-free WGSL the device is actually given) and which resource layout its DXIL was compiled from; when they differ, when a stage (entry point plus pipeline constants) is missing, or when the driver refuses it, the family compiles its WGSL as before. Tests also pin the wgpu version.
- Regenerate after a shader or layout change: `NANA_DXC_PATH=<dxcompiler.dll> cargo test -p nana-ui --lib --features gpu,hosted,full -- --ignored regenerate_precompiled_dxil --test-threads=1`. It builds each family's pipelines the way wgpu does on DX12, captures the HLSL naga writes, compiles it with DXC, checks it with the DXIL validator and writes `generated.rs`; `precompiled_families_paint_what_their_programs_paint` (`--ignored`, needs DX12) checks that DXIL and WGSL paint the same pixels.

### Shader IR cut per entry point

On every platform the framework's scene programs (quad, mesh, text, icon, chart, backdrop, dest, HostTexture, the motion evaluator, and the surface and text material programs) reach the device through `scene_paint::wgsl::Program`: a program is parsed and validated once, when first needed, and each entry point becomes a naga module of its own holding only the functions, types and constants it reaches (`naga::compact`), handed to wgpu as `ShaderSource::Naga`. Pipelines that share an entry point share its module; the device validates only the cut-down IR; wgpu-hal no longer clones, compacts and revalidates the whole module for every pipeline stage; and each pipeline keeps only its own stages' code (Metal writes MSL per pipeline, so modules stay alive). For example, the blit program has 39 entry points and 658 expressions, and one stage carries 5–424 of them; a mesh stage carries 53–737 (of 1 518), a chart stage 411–1 282 (of 2 593). A cut module no longer declares the overrides it does not use, and a pipeline sets only the constants its stage still declares (`Stage::constants`). A program that does not parse or validate here (the framework's do not; the tests compile each one) goes to the device whole as WGSL, which reports the error as before.

## Timeline metrics

`StartupStatus::timeline` records six key milestones measured from the host entry point (`run_runtime`): `splash_committed` → `ui_ready` → `takeover_requested` → `first_frame_submitted` → `handoff_completed` → `splash_released`. `StartupStatus::work` and `nana_diagnostics::framework::host` (`STARTUP_PHASE`, `SPLASH_OUTCOME`, `host.startup.device_request`, `host.startup.painter_build`, and others) also record the longest single blocking stretch on the event thread, device request time, painter compile time, and the number of live native resources (after handoff completes, `SplashWork::live_resources` returns strictly to zero).
