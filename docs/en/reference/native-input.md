# Windows GPU Shared Texture Input

The `native-import` feature of `nana-gpu` provides `NativeImportPool`: an external GPU producer writes a shared BGRA8 texture, and the host copies it on its own DX12 queue into a sampleable private texture. There is no CPU pixel readback or upload; there is still one copy within video memory, so this is not called fully zero-copy.

## Lifecycle and synchronization

1. Create the pool with the existing `GpuContext` and a pixel size. It allocates a fixed three shared input slots and one private output, and creates no second WGPU device/queue.
2. `try_acquire` claims a slot without blocking; when the pool is full, skip this frame. The target provides the adapter LUID, borrowed NT texture/fence handles, and release/ready values. The producer must select the same adapter.
3. The producer GPU-Waits on release, writes BGRA8, leaves the shared target and transitions back to COMMON, then Signals ready and Flushes; only after this is successfully queued may it call `unsafe into_ready`. If the Signal cannot be guaranteed to execute after the write, call `pool.retire()` first; never reopen the slot to the producer.
4. `stage(&mut FrameContext, ready)` GPU-Waits on ready on the host, copies, and transitions the shared source back to COMMON; release is Signaled after submit. Dropping a ready value or an unsubmitted frame also returns the slot on the GPU timeline, with no CPU wait.
5. Publish the output only after the frame is submitted. The returned texture is the latest image, overwritten in order on the same queue, not an immutable frame snapshot; when an immutable frame is needed, use leased frame exchange instead. The default sampled view is sRGB; the underlying BGRA8 UNORM is available for other sampling modes the producer declares.

Create a new pool for every new load, size change, or device generation change; new content must not overwrite output that still belongs to an old playback identity. Clear the producer's native-handle cache first, then release the old pool, so that an OS reusing a handle value does not hit the old texture by mistake. Old targets, ready values, and submitted GPU work keep ownership of their own resources. Non-DX12, wrong generation, retired pools, and cross-pool frames are all explicitly rejected.

On retirement, a WGPU queue tail marker after the native release Signal keeps the slot resources alive until the completion callback frees them; the normal per-frame path adds no submissions. The caller must keep polling the old `GpuContext`, consistent with the existing submission-completion callback contract. Stopping polling of the old device entirely after device replacement conservatively extends the old resources' lifetime.

`HostTexture` / `GpuTextureView` keep using the ordinary host texture path. The framework bridge is not responsible for the producer's EGL/OpenGL context, decoders, playback state, or application fallback policy.

## Verification

`cargo test -p nana-gpu --features native-import import::tests` covers real D3D11 writes → DX12 received pixels, steady-state resource reuse, pool full, dropping, retirement, and cross-device/cross-pool rejection. It depends on a native Windows/DX12 device; a compile check does not replace this kind of GPU verification.
