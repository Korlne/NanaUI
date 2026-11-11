//! The device's shader and pipeline cache, persisted by the host.
//!
//! Every pipeline built on a [`GpuContext`] whose device has a pipeline cache
//! goes through one cache the context owns: framework families, materials
//! and anything a renderer builds on the raw device alike. What the cache holds
//! depends on the backend — compiled shader bytecode on DX12 (so a warm start
//! skips the shader compiler), the driver's pipeline cache on Vulkan — and is
//! opaque here.
//!
//! The framework only hands bytes out and takes them back; where they live is
//! the host's decision:
//!
//! 1. Name the file by [`GpuContext::pipeline_cache_key`].
//! 2. Before building painters, give the bytes a previous run saved to
//!    [`GpuContext::load_pipeline_cache`].
//! 3. After building them (and again later, if it likes), save
//!    [`GpuContext::pipeline_cache_data`].
//!
//! Bytes that are not a cache, belong to another adapter or backend, were
//! truncated or corrupted, or were written by an older format are refused, and
//! the device keeps compiling as if nothing was loaded. Loading never fails a
//! device and never raises a backend validation error.

use std::fmt;
use std::sync::Arc;

use crate::{GpuBackend, GpuContext};

/// Which cache bytes a device can use. Hosts name the saved file by
/// [`Self::file_stem`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineCacheKey {
    backend: GpuBackend,
    vendor_id: u32,
    device_id: u32,
    stem: Arc<str>,
}

impl PipelineCacheKey {
    pub(crate) fn new(backend: GpuBackend, vendor_id: u32, device_id: u32) -> Option<Self> {
        let stem: Arc<str> = match backend {
            // Bytecode: the same on every adapter, so one file serves all.
            GpuBackend::Dx12 => "dx12".into(),
            // Driver state: one file per adapter.
            GpuBackend::Vulkan => format!("vulkan-{vendor_id:04x}-{device_id:04x}").into(),
            _ => return None,
        };
        Some(Self {
            backend,
            vendor_id,
            device_id,
            stem,
        })
    }

    pub const fn backend(&self) -> GpuBackend {
        self.backend
    }

    /// A file name without extension, distinct for every key whose bytes
    /// differ: `dx12`, or `vulkan-<vendor>-<device>` in hexadecimal.
    pub fn file_stem(&self) -> &str {
        &self.stem
    }
}

impl fmt::Display for PipelineCacheKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.stem)
    }
}

/// Where a host keeps cache bytes between runs, for hosts that hand the
/// framework a store instead of calling [`GpuContext::load_pipeline_cache`]
/// and [`GpuContext::pipeline_cache_data`] themselves. Both calls happen off
/// the event thread; neither may panic. A store that has nothing, or fails,
/// returns `None` or drops the bytes: the device then compiles as usual.
pub trait PipelineCacheStore: Send + Sync {
    fn load(&self, key: &PipelineCacheKey) -> Option<Vec<u8>>;
    fn save(&self, key: &PipelineCacheKey, bytes: &[u8]);
}

/// What [`GpuContext::load_pipeline_cache`] did with the bytes.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineCacheLoad {
    /// The cache now starts from the bytes. The backend may still drop parts
    /// it can no longer use (an updated driver, say); those compile again.
    Loaded,
    /// The bytes were refused; the device compiles as if nothing was loaded.
    Refused(PipelineCacheRefusal),
    /// The device has no pipeline cache: its backend has none, or the device
    /// was created without `PIPELINE_CACHE`.
    Unsupported,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineCacheRefusal {
    /// Not cache bytes, or from a format this build does not read.
    NotACache,
    /// Saved for another [`PipelineCacheKey`].
    OtherDevice,
    /// Truncated, extended or corrupted.
    Corrupt,
    /// The backend rejected them.
    Backend,
}

const MAGIC: [u8; 8] = *b"NANAPLC1";

/// The host-facing envelope: magic, key stem, payload length and checksum,
/// then the backend's own bytes (which carry wgpu's header and validation).
pub(crate) fn wrap(key: &PipelineCacheKey, payload: &[u8]) -> Vec<u8> {
    let stem = key.file_stem().as_bytes();
    let mut out = Vec::with_capacity(8 + 4 + stem.len() + 8 + 8 + payload.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&(stem.len() as u32).to_le_bytes());
    out.extend_from_slice(stem);
    out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    out.extend_from_slice(&checksum(payload).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

/// The payload of `bytes`, if they are an intact envelope for `key`.
pub(crate) fn unwrap<'a>(
    key: &PipelineCacheKey,
    bytes: &'a [u8],
) -> Result<&'a [u8], PipelineCacheRefusal> {
    let mut rest = bytes;
    let mut take = |len: usize| -> Option<&'a [u8]> {
        if rest.len() < len {
            return None;
        }
        let (head, tail) = rest.split_at(len);
        rest = tail;
        Some(head)
    };
    if take(8) != Some(&MAGIC[..]) {
        return Err(PipelineCacheRefusal::NotACache);
    }
    let corrupt = PipelineCacheRefusal::Corrupt;
    let stem_len = u32::from_le_bytes(take(4).ok_or(corrupt)?.try_into().unwrap()) as usize;
    let stem = take(stem_len).ok_or(corrupt)?;
    if stem != key.file_stem().as_bytes() {
        return Err(PipelineCacheRefusal::OtherDevice);
    }
    let len = u64::from_le_bytes(take(8).ok_or(corrupt)?.try_into().unwrap());
    let sum = u64::from_le_bytes(take(8).ok_or(corrupt)?.try_into().unwrap());
    let payload = rest;
    if payload.len() as u64 != len || checksum(payload) != sum {
        return Err(corrupt);
    }
    Ok(payload)
}

/// FNV-1a over 8-byte words with a SplitMix64 finish: stable across builds
/// and toolchains, which `std`'s hashers are not.
fn checksum(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64 ^ bytes.len() as u64;
    let (words, rest) = bytes.as_chunks::<8>();
    for word in words {
        hash = (hash ^ u64::from_le_bytes(*word))
            .wrapping_mul(0x0000_0100_0000_01b3)
            .rotate_left(23);
    }
    let mut last = [0u8; 8];
    last[..rest.len()].copy_from_slice(rest);
    hash = (hash ^ u64::from_le_bytes(last)).wrapping_mul(0x0000_0100_0000_01b3);
    hash = (hash ^ (hash >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash = (hash ^ (hash >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^ (hash >> 31)
}

/// Creates the device's cache from `data` and makes it the device default.
/// `None` when the backend refused it (reported through an error scope, so
/// it never reaches the uncaptured-error handler).
pub(crate) fn create(device: &wgpu::Device, data: Option<&[u8]>) -> Option<wgpu::PipelineCache> {
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    // SAFETY: wgpu validates the header it wrote (adapter, backend, version
    // and the backend's validation key) and falls back on anything it cannot
    // use; the DX12 and Vulkan payloads behind it are checked by the fork and
    // the driver respectively. The envelope checksum has already refused
    // bytes damaged since they were saved.
    let cache = unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("nana-gpu.pipeline-cache"),
            data,
            fallback: true,
        })
    };
    let failed = resolve(scope.pop()).is_none_or(|error| error.is_some());
    if failed {
        return None;
    }
    device.set_default_pipeline_cache(Some(&cache));
    Some(cache)
}

/// Resolves a future WGPU completes immediately on native backends. `None`
/// if it did not.
fn resolve<F: Future>(future: F) -> Option<F::Output> {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    match future.as_mut().poll(&mut context) {
        std::task::Poll::Ready(output) => Some(output),
        std::task::Poll::Pending => None,
    }
}

impl GpuContext {
    /// Which saved bytes this device can use, or `None` when it has no
    /// pipeline cache to persist.
    pub fn pipeline_cache_key(&self) -> Option<PipelineCacheKey> {
        self.inner.pipeline_cache_key.clone()
    }

    /// Start the device's cache from bytes [`Self::pipeline_cache_data`]
    /// returned in an earlier run. Call it before building painters: it
    /// replaces the cache, and what the old one collected is not carried over.
    pub fn load_pipeline_cache(&self, bytes: &[u8]) -> PipelineCacheLoad {
        let Some(key) = &self.inner.pipeline_cache_key else {
            return PipelineCacheLoad::Unsupported;
        };
        let payload = match unwrap(key, bytes) {
            Ok(payload) => payload,
            Err(refusal) => return PipelineCacheLoad::Refused(refusal),
        };
        let device = &self.inner.device;
        let mut slot = self
            .inner
            .pipeline_cache
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match create(device, Some(payload)) {
            Some(cache) => {
                *slot = Some(cache);
                PipelineCacheLoad::Loaded
            }
            // The cache the device had stays its default.
            None => PipelineCacheLoad::Refused(PipelineCacheRefusal::Backend),
        }
    }

    /// [`Self::load_pipeline_cache`] with what `store` has for this device.
    pub fn restore_pipeline_cache(&self, store: &dyn PipelineCacheStore) -> PipelineCacheLoad {
        let Some(key) = &self.inner.pipeline_cache_key else {
            return PipelineCacheLoad::Unsupported;
        };
        match store.load(key) {
            Some(bytes) => self.load_pipeline_cache(&bytes),
            None => PipelineCacheLoad::Refused(PipelineCacheRefusal::NotACache),
        }
    }

    /// Save [`Self::pipeline_cache_data`] to `store`. Whether there was
    /// anything to save.
    pub fn persist_pipeline_cache(&self, store: &dyn PipelineCacheStore) -> bool {
        let (Some(key), Some(bytes)) = (&self.inner.pipeline_cache_key, self.pipeline_cache_data())
        else {
            return false;
        };
        store.save(key, &bytes);
        true
    }

    /// The cache's bytes for the host to save under
    /// [`Self::pipeline_cache_key`], or `None` when the device has no cache.
    pub fn pipeline_cache_data(&self) -> Option<Vec<u8>> {
        let key = self.inner.pipeline_cache_key.as_ref()?;
        let payload = self
            .inner
            .pipeline_cache
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()?
            .get_data()?;
        Some(wrap(key, &payload))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> PipelineCacheKey {
        PipelineCacheKey::new(GpuBackend::Vulkan, 0x10de, 0x2d04).unwrap()
    }

    #[test]
    fn keys_name_their_file() {
        assert_eq!(key().file_stem(), "vulkan-10de-2d04");
        let dx12 = PipelineCacheKey::new(GpuBackend::Dx12, 0x10de, 0x2d04).unwrap();
        assert_eq!(dx12.file_stem(), "dx12");
        assert!(PipelineCacheKey::new(GpuBackend::Gl, 1, 2).is_none());
    }

    #[test]
    fn envelopes_round_trip() {
        let wrapped = wrap(&key(), b"payload");
        assert_eq!(unwrap(&key(), &wrapped), Ok(&b"payload"[..]));
    }

    #[test]
    fn envelopes_refuse_what_they_cannot_vouch_for() {
        let wrapped = wrap(&key(), b"payload bytes");
        assert_eq!(
            unwrap(&key(), b"not a cache at all"),
            Err(PipelineCacheRefusal::NotACache)
        );
        let other = PipelineCacheKey::new(GpuBackend::Vulkan, 0x1002, 1).unwrap();
        assert_eq!(
            unwrap(&other, &wrapped),
            Err(PipelineCacheRefusal::OtherDevice)
        );
        assert_eq!(
            unwrap(&key(), &wrapped[..wrapped.len() - 1]),
            Err(PipelineCacheRefusal::Corrupt)
        );
        let mut flipped = wrapped.clone();
        *flipped.last_mut().unwrap() ^= 1;
        assert_eq!(unwrap(&key(), &flipped), Err(PipelineCacheRefusal::Corrupt));
        assert_eq!(
            unwrap(&key(), &wrapped[..10]),
            Err(PipelineCacheRefusal::Corrupt)
        );
    }
}
