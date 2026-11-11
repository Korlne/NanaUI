//! The host-persisted shader and pipeline cache, on a real device.

use std::sync::Mutex;

use nana_gpu::__framework;
use nana_gpu::{
    GpuContext, PipelineCacheKey, PipelineCacheLoad, PipelineCacheRefusal, PipelineCacheStore,
};

fn context(pipeline_cache: bool) -> GpuContext {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::from_env().unwrap_or_default(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
        &instance, None,
    ))
    .expect("pipeline cache tests require a WGPU adapter");
    let required_features = if pipeline_cache {
        adapter.features() & wgpu::Features::PIPELINE_CACHE
    } else {
        wgpu::Features::empty()
    };
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_features,
        ..Default::default()
    }))
    .expect("pipeline cache tests require a WGPU device");
    __framework::adopt(adapter, device, queue)
}

/// A pipeline built the way every framework family builds one: without
/// naming a cache, so it goes through the device default.
fn build_pipeline(gpu: &GpuContext, color: f32) {
    let device = __framework::device(gpu);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("pipeline cache test shader"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "@vertex fn vs() -> @builtin(position) vec4<f32> {{ return vec4<f32>(0.0); }}
                 @fragment fn fs() -> @location(0) vec4<f32> {{ return vec4<f32>({color:?}); }}"
            )
            .into(),
        ),
    });
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let _pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("pipeline cache test pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs"),
            targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
            compilation_options: Default::default(),
        }),
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    });
    assert!(pollster::block_on(scope.pop()).is_none());
}

#[derive(Default)]
struct MemoryStore(Mutex<Option<(PipelineCacheKey, Vec<u8>)>>);

impl PipelineCacheStore for MemoryStore {
    fn load(&self, key: &PipelineCacheKey) -> Option<Vec<u8>> {
        let saved = self.0.lock().unwrap();
        saved
            .as_ref()
            .filter(|(saved, _)| saved == key)
            .map(|(_, bytes)| bytes.clone())
    }
    fn save(&self, key: &PipelineCacheKey, bytes: &[u8]) {
        *self.0.lock().unwrap() = Some((key.clone(), bytes.to_vec()));
    }
}

#[test]
fn a_device_without_a_cache_reports_it_and_compiles_as_before() {
    let gpu = context(false);
    assert_eq!(gpu.pipeline_cache_key(), None);
    assert_eq!(gpu.pipeline_cache_data(), None);
    assert_eq!(
        gpu.load_pipeline_cache(b"anything"),
        PipelineCacheLoad::Unsupported
    );
    build_pipeline(&gpu, 0.25);
}

#[test]
fn cache_bytes_carry_over_to_a_new_device() {
    let first = context(true);
    let Some(key) = first.pipeline_cache_key() else {
        // This backend keeps no pipeline cache.
        return;
    };
    build_pipeline(&first, 0.5);
    let store = MemoryStore::default();
    assert!(first.persist_pipeline_cache(&store));
    let saved = store.load(&key).expect("saved bytes");

    let second = context(true);
    assert_eq!(
        second.restore_pipeline_cache(&store),
        PipelineCacheLoad::Loaded
    );
    // The same pipeline again, and one the saved cache never saw.
    build_pipeline(&second, 0.5);
    build_pipeline(&second, 0.75);
    let grown = second.pipeline_cache_data().expect("cache bytes");
    if key.backend() == nana_gpu::GpuBackend::Dx12 {
        // Every compiled stage is kept, so the bytes only grow.
        assert!(grown.len() > saved.len());
    }
}

#[test]
fn bad_bytes_are_refused_and_the_device_keeps_compiling() {
    let gpu = context(true);
    let Some(_) = gpu.pipeline_cache_key() else {
        return;
    };
    build_pipeline(&gpu, 0.5);
    let good = gpu.pipeline_cache_data().expect("cache bytes");

    let fresh = context(true);
    assert_eq!(
        fresh.load_pipeline_cache(b"not a pipeline cache"),
        PipelineCacheLoad::Refused(PipelineCacheRefusal::NotACache)
    );
    assert_eq!(
        fresh.load_pipeline_cache(&good[..good.len() - 3]),
        PipelineCacheLoad::Refused(PipelineCacheRefusal::Corrupt)
    );
    let mut flipped = good.clone();
    let last = flipped.len() - 1;
    flipped[last] ^= 0x5a;
    assert_eq!(
        fresh.load_pipeline_cache(&flipped),
        PipelineCacheLoad::Refused(PipelineCacheRefusal::Corrupt)
    );
    build_pipeline(&fresh, 0.5);
    assert!(fresh.pipeline_cache_data().is_some());
}
