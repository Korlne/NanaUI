# nana-app-icon

English · [简体中文](README.zh-CN.md)

NanaUI's default application icon, plus PNG, ICO, and ICNS encoding and icon embedding for Windows executables.

When an application has not registered an icon, the host uses the built-in Nana silhouette here as the window and Dock icon, with every size scaled from the same source image. `resolved_application_icon` picks, in order, the per-window icon, the registered icon, and the default icon. Calling `embed_windows` in `build.rs` compiles the icon into a Windows executable; on non-Windows targets it does nothing.

The icon registration API is in [`nana-ui-platform`](../nana-ui-platform), and the package directory layout is in [`nana-packager`](../nana-packager). Applications that register their own icon can turn on `nana-ui`'s `no-default-app-icon` to avoid linking the default icon.

## Example

`build.rs`, with `embed` turned on for `nana-app-icon` under `[build-dependencies]`:

```rust
fn main() {
    nana_app_icon::embed_windows();
}
```

## Features

- `runtime` (on by default): the default window icon and icon resolution, for hosts.
- `embed`: embeds the Windows icon resource from `build.rs`.

## Tests

```bash
cargo test -p nana-app-icon --all-features -- --test-threads=1
```

The tests under `runtime` change the process-wide registered icon, so they must run single-threaded.

See also [Packaging and distribution](../../docs/en/reference/packaging.md).
