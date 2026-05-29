//! Sidecar chunk writer for crash-safe terminal output storage.
//!
//! Terminal output is written as an append-only sequence of framed records
//! to a `.bin` sidecar file. Each record includes a monotonic sequence
//! identifier and a BLAKE3 hash of the payload for torn-write detection at
//! crash recovery time.
//!
//! ## Frame format
//!
//! ```text
//! [monotonic_id: u64 LE] [blake3_hash: [u8; 32]] [payload_len: u32 LE] [payload: bytes]
//! ```
//!
//! Total header: 44 bytes. Maximum payload per frame: 4 GiB (`u32::MAX`).

#![allow(clippy::missing_errors_doc)]

use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors from the sidecar chunk writer / reader.
#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    /// Underlying I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Payload exceeds the 4 GiB per-frame limit.
    #[error("payload exceeds 4 GiB limit")]
    PayloadTooLarge,
}

// ---------------------------------------------------------------------------
// ChunkRef
// ---------------------------------------------------------------------------

/// Reference to a successfully written chunk frame.
#[derive(Debug, Clone, Copy)]
pub struct ChunkRef {
    /// Monotonic sequence number assigned to this frame.
    pub monotonic_id: u64,
    /// BLAKE3 hash of the payload bytes.
    pub hash: [u8; 32],
}

// ---------------------------------------------------------------------------
// SidecarChunkWriter
// ---------------------------------------------------------------------------

/// Append-only binary chunk writer backed by a single file.
///
/// Each call to [`write_chunk`][Self::write_chunk] atomically appends one
/// framed record and flushes. Thread-safe via a [`parking_lot::Mutex`] around
/// the file handle.
pub struct SidecarChunkWriter {
    file: parking_lot::Mutex<std::fs::File>,
    next_id: AtomicU64,
}

impl SidecarChunkWriter {
    /// Open (or create) a chunk file at `path` in append mode.
    pub fn open(path: &Path) -> Result<Self, SidecarError> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        Ok(Self {
            file: parking_lot::Mutex::new(file),
            next_id: AtomicU64::new(0),
        })
    }

    /// Append one chunk record and flush.
    ///
    /// Returns a [`ChunkRef`] with the assigned monotonic ID and BLAKE3 hash.
    pub fn write_chunk(&self, payload: &[u8]) -> Result<ChunkRef, SidecarError> {
        let len = u32::try_from(payload.len()).map_err(|_| SidecarError::PayloadTooLarge)?;
        let hash = blake3::hash(payload);
        let hash_bytes = *hash.as_bytes();
        let monotonic_id = self.next_id.fetch_add(1, Ordering::SeqCst);

        // Build the complete frame in a local buffer for a single write call.
        let mut buf = Vec::with_capacity(44 + payload.len());
        buf.extend_from_slice(&monotonic_id.to_le_bytes());
        buf.extend_from_slice(&hash_bytes);
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(payload);

        let mut file = self.file.lock();
        file.write_all(&buf)?;
        file.flush()?;

        Ok(ChunkRef { monotonic_id, hash: hash_bytes })
    }

    /// `fsync` the file to durable storage.
    pub fn sync(&self) -> Result<(), SidecarError> {
        self.file.lock().sync_all().map_err(SidecarError::Io)
    }
}

// ---------------------------------------------------------------------------
// SidecarReader
// ---------------------------------------------------------------------------

/// Read back frames from a sidecar chunk file.
///
/// Iterates records until EOF or a hash mismatch (torn write detection).
pub struct SidecarReader {
    path: PathBuf,
}

impl SidecarReader {
    /// Create a reader for the chunk file at `path`.
    #[must_use]
    pub fn open(path: &Path) -> Self {
        Self { path: path.to_path_buf() }
    }

    /// Read all valid frames from the file.
    ///
    /// Returns `(monotonic_id, payload)` pairs in write order, stopping at
    /// the first incomplete or hash-mismatched frame.
    pub fn read_valid_chunks(&self) -> Result<Vec<(u64, Vec<u8>)>, SidecarError> {
        let mut file = std::fs::File::open(&self.path)?;
        let mut chunks = Vec::new();

        loop {
            // monotonic_id — EOF here is clean end-of-stream.
            let mut id_buf = [0u8; 8];
            match file.read_exact(&mut id_buf) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(SidecarError::Io(e)),
            }
            let monotonic_id = u64::from_le_bytes(id_buf);

            // blake3 hash
            let mut hash_buf = [0u8; 32];
            if file.read_exact(&mut hash_buf).is_err() {
                break; // torn header
            }

            // payload length
            let mut len_buf = [0u8; 4];
            if file.read_exact(&mut len_buf).is_err() {
                break; // torn header
            }
            let payload_len = u32::from_le_bytes(len_buf) as usize;

            // payload
            let mut payload = vec![0u8; payload_len];
            if file.read_exact(&mut payload).is_err() {
                break; // torn payload
            }

            // Integrity check — stop on mismatch (torn write).
            let computed = blake3::hash(&payload);
            if *computed.as_bytes() != hash_buf {
                break;
            }

            chunks.push((monotonic_id, payload));
        }

        Ok(chunks)
    }
}

// ---------------------------------------------------------------------------
// Crash recovery scan
// ---------------------------------------------------------------------------

/// Enumerate `.bin` sidecar files in `chunks_dir`.
///
/// Used at startup to identify chunk files that may belong to sessions
/// whose SQLite records were not flushed before a crash.
///
/// Returns an empty `Vec` when the directory does not exist.
#[must_use]
pub fn scan_for_recovery(chunks_dir: &Path) -> Vec<PathBuf> {
    if !chunks_dir.exists() {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(chunks_dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |ext| ext == "bin"))
        .collect()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_and_read_single_chunk() {
        let dir = TempDir::new().expect("tmpdir");
        let path = dir.path().join("pane.bin");
        let writer = SidecarChunkWriter::open(&path).expect("open writer");

        let payload = b"hello sidecar";
        let chunk_ref = writer.write_chunk(payload).expect("write");
        assert_eq!(chunk_ref.monotonic_id, 0);

        let reader = SidecarReader::open(&path);
        let chunks = reader.read_valid_chunks().expect("read");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].0, 0);
        assert_eq!(&chunks[0].1, payload.as_slice());
    }

    #[test]
    fn write_multiple_chunks_sequential_ids() {
        let dir = TempDir::new().expect("tmpdir");
        let path = dir.path().join("pane.bin");
        let writer = SidecarChunkWriter::open(&path).expect("open");

        for i in 0u64..5 {
            let r = writer
                .write_chunk(format!("payload {i}").as_bytes())
                .expect("write");
            assert_eq!(r.monotonic_id, i);
        }

        let chunks = SidecarReader::open(&path).read_valid_chunks().expect("read");
        assert_eq!(chunks.len(), 5);
        for (i, (id, payload)) in chunks.iter().enumerate() {
            assert_eq!(*id, i as u64);
            assert_eq!(payload, format!("payload {i}").as_bytes());
        }
    }

    #[test]
    fn torn_write_stops_at_hash_mismatch() {
        let dir = TempDir::new().expect("tmpdir");
        let path = dir.path().join("torn.bin");
        let writer = SidecarChunkWriter::open(&path).expect("open");

        writer.write_chunk(b"good chunk").expect("write good");

        // Append a frame with a zeroed (wrong) hash to simulate a torn write.
        {
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .expect("open for corruption");
            let id: u64 = 1;
            let bad_hash = [0u8; 32];
            let bad_payload = b"corrupt";
            let len = bad_payload.len() as u32;
            file.write_all(&id.to_le_bytes()).unwrap();
            file.write_all(&bad_hash).unwrap();
            file.write_all(&len.to_le_bytes()).unwrap();
            file.write_all(bad_payload).unwrap();
        }

        let chunks = SidecarReader::open(&path).read_valid_chunks().expect("read");
        assert_eq!(chunks.len(), 1, "only the valid frame should be returned");
        assert_eq!(&chunks[0].1, b"good chunk".as_slice());
    }

    #[test]
    fn scan_returns_empty_for_missing_dir() {
        let found = scan_for_recovery(Path::new("no-such-dir-xyz-bongt"));
        assert!(found.is_empty());
    }

    #[test]
    fn scan_finds_bin_files_only() {
        let dir = TempDir::new().expect("tmpdir");
        std::fs::write(dir.path().join("a.bin"), b"").expect("write a");
        std::fs::write(dir.path().join("b.bin"), b"").expect("write b");
        std::fs::write(dir.path().join("c.txt"), b"").expect("write c");

        let found = scan_for_recovery(dir.path());
        assert_eq!(found.len(), 2);
        for p in &found {
            assert_eq!(p.extension().unwrap(), "bin");
        }
    }
}
