//! NanaUI fork: the DX12 [`PipelineCache`](super::PipelineCache).
//!
//! It holds compiled shader bytecode (DXIL from DXC, DXBC from FXC) keyed by a
//! hash of everything that decides that output: the HLSL text naga wrote, the
//! entry point, the target profile (stage and shader model) and the compile
//! flags. A pipeline created with the cache looks its stages up there and only
//! runs the compiler on a miss, so a cache carried over from an earlier run
//! skips the compiler entirely.
//!
//! Bytecode does not depend on the adapter or driver, only on the compiler, so
//! the compiler's identity is part of the validation key wgpu-core checks
//! before data reaches [`ShaderBlobCache::load`], and the bytes are portable
//! between adapters.
//!
//! The format (after wgpu-core's header), little endian:
//!
//! ```text
//! magic    [u8; 8]  "NDXSHC01"
//! count    u32
//! entries  count × { key [u8; 16], age u32, len u32, checksum [u8; 16], bytes [u8; len] }
//! ```
//!
//! Anything that does not parse is dropped as a whole; an entry whose checksum
//! does not match is dropped alone. Either way the cache falls back to
//! compiling.

use alloc::{sync::Arc, vec::Vec};

use hashbrown::HashMap;

const MAGIC: [u8; 8] = *b"NDXSHC01";

/// Entries not used for this many saves in a row are not saved again, so the
/// bytes of shaders an application stopped building do not stay forever.
const MAX_AGE: u32 = 8;

/// A 128-bit content hash. Not cryptographic: the cache is the host's data,
/// and the hash only has to tell different compiler inputs apart.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(super) struct ShaderKey(pub(super) [u8; 16]);

#[derive(Debug)]
struct Entry {
    bytes: Arc<[u8]>,
    /// Saves since this entry was last used; 0 once used in this process.
    age: u32,
}

#[derive(Debug, Default)]
pub(super) struct ShaderBlobCache {
    entries: HashMap<ShaderKey, Entry>,
}

impl ShaderBlobCache {
    /// The cache `data` describes; empty when `data` is not a cache.
    pub(super) fn load(data: &[u8]) -> Self {
        Self::parse(data).unwrap_or_default()
    }

    fn parse(data: &[u8]) -> Option<Self> {
        let mut reader = Reader(data);
        if reader.take(8)? != MAGIC {
            return None;
        }
        let count = reader.u32()?;
        let mut entries = HashMap::new();
        for _ in 0..count {
            let key = ShaderKey(reader.take(16)?.try_into().ok()?);
            let age = reader.u32()?.saturating_add(1);
            let len = reader.u32()? as usize;
            let checksum: [u8; 16] = reader.take(16)?.try_into().ok()?;
            let bytes = reader.take(len)?;
            if checksum != hash_bytes(bytes) {
                continue;
            }
            entries.insert(
                key,
                Entry {
                    bytes: bytes.into(),
                    age,
                },
            );
        }
        if !reader.0.is_empty() {
            return None;
        }
        Some(Self { entries })
    }

    pub(super) fn get(&mut self, key: &ShaderKey) -> Option<Arc<[u8]>> {
        let entry = self.entries.get_mut(key)?;
        entry.age = 0;
        Some(Arc::clone(&entry.bytes))
    }

    pub(super) fn insert(&mut self, key: ShaderKey, bytes: &[u8]) {
        self.entries.insert(
            key,
            Entry {
                bytes: bytes.into(),
                age: 0,
            },
        );
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }

    pub(super) fn serialize(&self) -> Vec<u8> {
        let kept = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.age <= MAX_AGE)
            .collect::<Vec<_>>();
        let size = 12 + kept
            .iter()
            .map(|(_, entry)| 40 + entry.bytes.len())
            .sum::<usize>();
        let mut out = Vec::with_capacity(size);
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&(kept.len() as u32).to_le_bytes());
        for (key, entry) in kept {
            out.extend_from_slice(&key.0);
            out.extend_from_slice(&entry.age.to_le_bytes());
            out.extend_from_slice(&(entry.bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(&hash_bytes(&entry.bytes));
            out.extend_from_slice(&entry.bytes);
        }
        out
    }
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, len: usize) -> Option<&'a [u8]> {
        if self.0.len() < len {
            return None;
        }
        let (head, tail) = self.0.split_at(len);
        self.0 = tail;
        Some(head)
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
}

/// Two independent 64-bit lanes over 8-byte words, each finished with the
/// SplitMix64 avalanche. Every field is length-prefixed by [`KeyHasher::field`],
/// so concatenations cannot collide.
pub(super) struct KeyHasher {
    a: u64,
    b: u64,
    len: u64,
}

impl KeyHasher {
    pub(super) fn new() -> Self {
        Self {
            a: 0x243f_6a88_85a3_08d3,
            b: 0x1319_8a2e_0370_7344,
            len: 0,
        }
    }

    pub(super) fn field(&mut self, bytes: &[u8]) -> &mut Self {
        self.word(bytes.len() as u64);
        let mut chunks = bytes.chunks_exact(8);
        for chunk in &mut chunks {
            self.word(u64::from_le_bytes(chunk.try_into().unwrap()));
        }
        let rest = chunks.remainder();
        if !rest.is_empty() {
            let mut last = [0u8; 8];
            last[..rest.len()].copy_from_slice(rest);
            self.word(u64::from_le_bytes(last));
        }
        self
    }

    fn word(&mut self, word: u64) {
        self.len = self.len.wrapping_add(1);
        self.a = (self.a ^ word)
            .wrapping_mul(0x9e37_79b9_7f4a_7c15)
            .rotate_left(29);
        self.b = (self.b ^ word.rotate_left(32))
            .wrapping_mul(0xc2b2_ae3d_27d4_eb4f)
            .rotate_left(31)
            .wrapping_add(self.a);
    }

    pub(super) fn finish(&self) -> [u8; 16] {
        let a = mix(self.a ^ self.len);
        let b = mix(self.b.wrapping_add(a));
        let mut out = [0u8; 16];
        out[..8].copy_from_slice(&a.to_le_bytes());
        out[8..].copy_from_slice(&b.to_le_bytes());
        out
    }
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn hash_bytes(bytes: &[u8]) -> [u8; 16] {
    KeyHasher::new().field(bytes).finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: u8) -> ShaderKey {
        ShaderKey(KeyHasher::new().field(&[n]).finish())
    }

    #[test]
    fn round_trips() {
        let mut cache = ShaderBlobCache::default();
        cache.insert(key(1), b"first");
        cache.insert(key(2), b"second shader");
        let mut loaded = ShaderBlobCache::load(&cache.serialize());
        assert_eq!(loaded.len(), 2);
        assert_eq!(&*loaded.get(&key(1)).unwrap(), b"first");
        assert_eq!(&*loaded.get(&key(2)).unwrap(), b"second shader");
    }

    #[test]
    fn rejects_what_does_not_parse() {
        let mut cache = ShaderBlobCache::default();
        cache.insert(key(1), b"first");
        let data = cache.serialize();
        assert_eq!(ShaderBlobCache::load(&data[..data.len() - 1]).len(), 0);
        assert_eq!(ShaderBlobCache::load(b"garbage").len(), 0);
        let mut extended = data.clone();
        extended.push(0);
        assert_eq!(ShaderBlobCache::load(&extended).len(), 0);
    }

    #[test]
    fn drops_a_corrupt_entry_alone() {
        let mut cache = ShaderBlobCache::default();
        cache.insert(key(1), b"first");
        cache.insert(key(2), b"second");
        let mut data = cache.serialize();
        let last = data.len() - 1;
        data[last] ^= 0xff;
        let loaded = ShaderBlobCache::load(&data);
        assert_eq!(loaded.len(), 1);
    }

    #[test]
    fn unused_entries_age_out() {
        let mut cache = ShaderBlobCache::default();
        cache.insert(key(1), b"stale");
        cache.insert(key(2), b"used");
        let mut data = cache.serialize();
        for _ in 0..=MAX_AGE {
            let mut loaded = ShaderBlobCache::load(&data);
            loaded.get(&key(2)).unwrap();
            data = loaded.serialize();
        }
        let mut loaded = ShaderBlobCache::load(&data);
        assert!(loaded.get(&key(1)).is_none());
        assert!(loaded.get(&key(2)).is_some());
    }

    #[test]
    fn fields_are_delimited() {
        let ab = KeyHasher::new().field(b"ab").field(b"c").finish();
        let a_bc = KeyHasher::new().field(b"a").field(b"bc").finish();
        assert_ne!(ab, a_bc);
    }
}
