//! Tests for NanaUI's changes to its wgpu fork (`third_party/wgpu`).
//!
//! The library only holds what the GPU tests share. A test that finds no
//! adapter for its backend skips; with `NANA_REQUIRE_WGPU_FORK_TESTS=1` it
//! fails instead, so CI on a machine that has the backend cannot pass by
//! skipping.

/// Whether a missing adapter must fail the test instead of skipping it.
pub fn adapter_required() -> bool {
    std::env::var_os("NANA_REQUIRE_WGPU_FORK_TESTS").is_some_and(|value| value == "1")
}

/// Skip (or, when required, fail) a test that has no adapter to run on.
pub fn no_adapter(test: &str, backend: wgpu::Backends) {
    assert!(
        !adapter_required(),
        "{test}: no {backend:?} adapter, and NANA_REQUIRE_WGPU_FORK_TESTS=1"
    );
    println!("{test}: skipped, no {backend:?} adapter");
}

/// An adapter and device on `backend` with `features` (those the adapter
/// offers), or `None` when the backend has no adapter here.
pub fn device(
    backend: wgpu::Backends,
    features: wgpu::Features,
) -> Option<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: backend,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features: adapter.features() & features,
        ..Default::default()
    }))
    .ok()?;
    Some((adapter, device, queue))
}
