//! CPU cost of a frame's uploads, per path.
//!
//! Each frame writes `writes` runs of `bytes` bytes to distinct destination
//! buffers and submits. What is timed is the thread's own CPU (Windows
//! `QueryThreadCycleTime`, so a wait on the GPU costs nothing) for the writes
//! plus the submit, and separately for the wait on the frame, which runs the
//! callbacks that return staging to the ring. Paths:
//!
//! - `none`: the frame without uploads (what every frame pays anyway).
//!
//! Every frame also clears a small target in a render pass, so each submit
//! carries real GPU work and the paths are measured as what they add to it.
//! - `ring`: `FrameUploadHandle::write_buffer` — the `UploadRing` path.
//! - `text`: one `queue.write_buffer_with` into a staging ring, then one
//!   `copy_buffer_to_buffer` per run in the frame's encoder (the text
//!   pipeline's instance/index path).
//! - `queue`: one `queue.write_buffer` per run.
//! - `copy`: only the `copy_buffer_to_buffer`s of `text`, from a buffer
//!   already on the GPU: what the copy commands cost without any staging.
//! - `memcpy`: the same bytes copied into memory that stays mapped, with no
//!   wgpu call: the floor a persistently mapped upload path could reach.
//!
//! ```bash
//! WGPU_BACKEND=dx12 WGPU_ADAPTER_NAME=nvidia cargo run --release -p nana-gpu --example upload-cost
//! ```

use std::time::Instant;

use nana_gpu::__framework;

const FRAMES: usize = 2000;
const WARMUP: usize = 200;

#[cfg(windows)]
fn thread_cycles() -> u64 {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentThread() -> isize;
        fn QueryThreadCycleTime(thread: isize, cycles: *mut u64) -> i32;
    }
    let mut cycles = 0;
    // SAFETY: the pseudo-handle of the calling thread and a valid out pointer.
    unsafe { QueryThreadCycleTime(GetCurrentThread(), &mut cycles) };
    cycles
}

#[cfg(not(windows))]
fn thread_cycles() -> u64 {
    0
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Path {
    None,
    Ring,
    Text,
    Queue,
    Copy,
    Memcpy,
}

fn main() {
    let instance =
        wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
    let adapter = pollster::block_on(wgpu::util::initialize_adapter_from_env_or_default(
        &instance, None,
    ))
    .expect("an adapter");
    let info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("upload-cost"),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .expect("a device");
    let gpu = __framework::adopt(adapter, device.clone(), queue.clone());
    println!(
        "adapter: {} ({:?}, {:?})",
        info.name, info.backend, info.device_type
    );
    println!(
        "{:<10} {:>6} {:>7} {:>12} {:>12} {:>12} {:>12} {:>10}",
        "path", "writes", "bytes", "record kcyc", "submit kcyc", "poll kcyc", "total kcyc", "submit us"
    );
    for (writes, bytes) in [(4usize, 256usize), (32, 1024), (8, 64 * 1024)] {
        for path in [
            Path::None,
            Path::Ring,
            Path::Text,
            Path::Queue,
            Path::Copy,
            Path::Memcpy,
        ] {
            run(&gpu, &device, &queue, path, writes, bytes);
        }
        println!();
    }
}

fn run(
    gpu: &nana_gpu::GpuContext,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    path: Path,
    writes: usize,
    bytes: usize,
) {
    let targets: Vec<wgpu::Buffer> = (0..writes)
        .map(|_| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("upload-cost target"),
                size: bytes as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        })
        .collect();
    let frame_bytes = (writes * bytes) as u64;
    let ring_size = (frame_bytes * 4).max(64 * 1024);
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("upload-cost staging ring"),
        size: ring_size,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // Mapped once and never unmapped: write-combined upload memory, where a
    // persistently mapped path would write.
    let mapped = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("upload-cost mapped"),
        size: frame_bytes,
        usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: true,
    });
    let mut mapped_view = mapped.slice(..).get_mapped_range_mut().expect("mapped");
    // Every frame clears a target, as a real one draws: what the paths add
    // is measured against a frame that already submits GPU work.
    let color = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("upload-cost target"),
        size: wgpu::Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let color = color.create_view(&wgpu::TextureViewDescriptor::default());
    let data: Vec<u8> = (0..bytes).map(|i| i as u8).collect();
    let mut cursor = 0u64;

    let mut record = Vec::with_capacity(FRAMES);
    let mut submit = Vec::with_capacity(FRAMES);
    let mut poll = Vec::with_capacity(FRAMES);
    let mut wall = Vec::with_capacity(FRAMES);
    for frame in 0..WARMUP + FRAMES {
        let mut recording = gpu.begin_frame("upload-cost");
        let started = Instant::now();
        let before = thread_cycles();
        match path {
            Path::None => {}
            Path::Ring => {
                let uploads = __framework::frame_uploads(&recording);
                for target in &targets {
                    uploads.write_buffer(target, 0, &data);
                }
            }
            Path::Text => {
                if cursor + frame_bytes > ring_size {
                    cursor = 0;
                }
                let at = cursor;
                cursor += frame_bytes;
                {
                    let mut view = queue
                        .write_buffer_with(
                            &staging,
                            at,
                            wgpu::BufferSize::new(frame_bytes).expect("nonzero"),
                        )
                        .expect("staging");
                    for run in 0..writes {
                        view.slice(run * bytes..(run + 1) * bytes)
                            .copy_from_slice(&data);
                    }
                }
                let encoder = __framework::encoder(&mut recording);
                for (run, target) in targets.iter().enumerate() {
                    encoder.copy_buffer_to_buffer(
                        &staging,
                        at + (run * bytes) as u64,
                        target,
                        0,
                        bytes as u64,
                    );
                }
            }
            Path::Queue => {
                for target in &targets {
                    queue.write_buffer(target, 0, &data);
                }
            }
            Path::Copy => {
                let encoder = __framework::encoder(&mut recording);
                for (run, target) in targets.iter().enumerate() {
                    encoder.copy_buffer_to_buffer(
                        &staging,
                        (run * bytes) as u64,
                        target,
                        0,
                        bytes as u64,
                    );
                }
            }
            Path::Memcpy => {
                for run in 0..writes {
                    mapped_view
                        .slice(run * bytes..(run + 1) * bytes)
                        .copy_from_slice(&data);
                }
            }
        }
        {
            let encoder = __framework::encoder(&mut recording);
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("upload-cost clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
        }
        let recorded = thread_cycles();
        let submission = recording.submit();
        let submitted = thread_cycles();
        let submit_wall = started.elapsed();
        // Wait for the frame: a blocked thread spends no cycles, so this is
        // the completion callbacks (chunk back to the ring, frame slot).
        let _ = device.poll(wgpu::PollType::Wait {
            submission_index: Some(__framework::submission_index(&submission).clone()),
            timeout: None,
        });
        let after = thread_cycles();
        let polled = submitted;
        if frame >= WARMUP {
            record.push(recorded - before);
            submit.push(submitted - before);
            poll.push(after - polled);
            wall.push(submit_wall.as_secs_f64() * 1e6);
        }
    }
    drop(mapped_view);
    let median = |values: &mut Vec<u64>| {
        values.sort_unstable();
        values[values.len() / 2]
    };
    let r = median(&mut record);
    let s = median(&mut submit);
    let p = median(&mut poll);
    wall.sort_by(f64::total_cmp);
    println!(
        "{:<10} {:>6} {:>7} {:>12.1} {:>12.1} {:>12.1} {:>12.1} {:>10.1}",
        format!("{path:?}").to_lowercase(),
        writes,
        bytes,
        r as f64 / 1000.0,
        s as f64 / 1000.0,
        p as f64 / 1000.0,
        (s + p) as f64 / 1000.0,
        wall[wall.len() / 2],
    );
}
