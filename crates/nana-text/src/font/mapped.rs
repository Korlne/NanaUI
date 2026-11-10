//! System font files mapped into memory rather than read. The only file
//! allowed to name `memmap2`.
//!
//! A platform's font files are large and mostly unused by any one
//! application: PingFang, Hiragino and Apple Color Emoji are 60 to 190 MB
//! each, and a line of text reads a few tables and a handful of glyphs from
//! one. Read whole, five lines of mixed text held 1.28 GB of heap on macOS.
//! Mapped, the bytes stay in the page cache the system already shares between
//! processes, and only the pages a face's tables are read from are resident.

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use super::system::{FontBlob, font_blob};

struct Mapped(memmap2::Mmap);

impl AsRef<[u8]> for Mapped {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// The bytes of the system font file at `path`: mapped, or read when the
/// file cannot be (a file system without mapping, an empty file). `None`
/// when it cannot be opened either.
pub(super) fn system_file(path: &Path) -> Option<FontBlob> {
    let mapped = File::open(path).and_then(|file| {
        // SAFETY: the mapping is private and read-only, and nothing here
        // writes the file. What the type cannot rule out is another process
        // shortening the file while it is mapped, which faults on the next
        // read of a page past its new end. Platforms replace an installed
        // font by writing a new file, not by truncating the old one in
        // place, and every text stack that maps system fonts (`fontdb`
        // itself, behind the same call) relies on that. Files an application
        // registers by path make no such promise and are still read.
        unsafe { memmap2::Mmap::map(&file) }
    });
    match mapped {
        Ok(map) if !map.is_empty() => Some(Arc::new(Mapped(map))),
        _ => std::fs::read(path).ok().map(font_blob),
    }
}
