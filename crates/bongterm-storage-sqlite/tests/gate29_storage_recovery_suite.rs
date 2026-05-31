//! Gate #29 — `storage_recovery_suite` (Phase-1 exit, design-spec §6.1).
//!
//! "SQLite WAL + sidecar chunk recovery handles **torn write** + **checksum
//! mismatch** + **corrupt DB** without silent transcript fabrication."
//!
//! This integration test exercises the REAL recovery code paths in
//! `bongterm-storage-sqlite`:
//!
//! * (a) torn write — `SidecarReader::read_valid_chunks` stops at the first
//!   frame whose stored hash does not match a freshly computed BLAKE3 of its
//!   payload (a trailing frame appended with a bogus hash, as if a crash tore
//!   the write mid-flight).
//! * (b) checksum mismatch — a *later* frame's payload is mutated in place
//!   after a clean write, so its previously-stored hash no longer matches; the
//!   reader returns only the valid prefix (the earlier good frames), never the
//!   corrupted one.
//! * (c) corrupt DB — a file of clear garbage where a SQLite database is
//!   expected; opening + migrating the store surfaces an error WITHOUT
//!   panicking and WITHOUT returning any fabricated rows. SQLite is a
//!   reconstructable cache (CLAUDE.md §Source-of-truth split) — corruption must
//!   be surfaced, never silently fabricated.
//!
//! Anti-gaming: every assertion drives production code. None pass "by
//! construction" — (a)/(b) re-hash real payloads through the reader, (c) opens
//! a real `SqliteStore` against on-disk garbage and inspects the real
//! `Result`/row count.

use std::io::{Seek, SeekFrom, Write};

use bongterm_storage_api::{AgentRunId, MigrationRunner, TranscriptRepo};
use bongterm_storage_sqlite::SqliteStore;
use bongterm_storage_sqlite::sidecar::{SidecarChunkWriter, SidecarReader};
use tempfile::TempDir;
use uuid::Uuid;

/// Frame header size: `monotonic_id` (8) + BLAKE3 hash (32) + `payload_len` (4).
const FRAME_HEADER_LEN: u64 = 44;

// ---------------------------------------------------------------------------
// (a) Torn write — a trailing frame with a wrong (zeroed) hash is rejected.
// ---------------------------------------------------------------------------
#[test]
fn torn_write_returns_only_valid_prefix() {
    let dir = TempDir::new().expect("tmpdir");
    let path = dir.path().join("torn.bin");

    // Write two good frames through the real writer.
    let writer = SidecarChunkWriter::open(&path).expect("open writer");
    writer.write_chunk(b"alpha").expect("write alpha");
    writer.write_chunk(b"bravo").expect("write bravo");
    writer.sync().expect("sync");

    // Simulate a crash that appended a third frame whose payload never matched
    // its (zeroed) hash — a torn/partial write left on disk.
    {
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open for tear");
        let id: u64 = 2;
        let bad_hash = [0u8; 32];
        let payload = b"torn-tail";
        let len = u32::try_from(payload.len()).unwrap();
        f.write_all(&id.to_le_bytes()).unwrap();
        f.write_all(&bad_hash).unwrap();
        f.write_all(&len.to_le_bytes()).unwrap();
        f.write_all(payload).unwrap();
        f.flush().unwrap();
    }

    // Real recovery path: the reader must stop at the torn frame.
    let chunks = SidecarReader::open(&path)
        .read_valid_chunks()
        .expect("read valid chunks");

    println!("(a) torn write -> recovered {} frame(s)", chunks.len());
    assert_eq!(
        chunks.len(),
        2,
        "only the two hash-verified frames may be recovered; the torn tail is dropped"
    );
    assert_eq!(&chunks[0].1, b"alpha", "first good frame survives");
    assert_eq!(&chunks[1].1, b"bravo", "second good frame survives");
    // No fabrication: the bogus payload must never appear in the recovered set.
    assert!(
        chunks.iter().all(|(_, p)| p.as_slice() != b"torn-tail"),
        "the torn payload must not be fabricated into the recovered stream"
    );
}

// ---------------------------------------------------------------------------
// (b) Checksum mismatch — a later frame's payload is mutated in place.
// ---------------------------------------------------------------------------
#[test]
fn checksum_mismatch_stops_at_corrupt_frame() {
    let dir = TempDir::new().expect("tmpdir");
    let path = dir.path().join("checksum.bin");

    // Three good frames, each a fixed 4-byte payload so offsets are exact.
    let writer = SidecarChunkWriter::open(&path).expect("open writer");
    writer.write_chunk(b"one_").expect("write one");
    writer.write_chunk(b"two_").expect("write two");
    writer.write_chunk(b"thre").expect("write three");
    writer.sync().expect("sync");
    drop(writer);

    // Corrupt the SECOND frame's payload in place (not the first), so the
    // recovered prefix is non-empty and we prove the reader stops *at* the
    // corruption while keeping everything before it.
    //
    // Layout: [frame0: 44 + 4][frame1: 44 + 4][frame2: ...]
    // Frame 1 payload begins at: (44 + 4) + 44 = 92.
    let frame1_payload_off = (FRAME_HEADER_LEN + 4) + FRAME_HEADER_LEN;
    {
        let mut f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open for in-place corruption");
        f.seek(SeekFrom::Start(frame1_payload_off))
            .expect("seek to frame1 payload");
        // Flip one payload byte; the stored hash now disagrees.
        f.write_all(b"X").expect("mutate one payload byte");
        f.flush().unwrap();
    }

    let chunks = SidecarReader::open(&path)
        .read_valid_chunks()
        .expect("read valid chunks");

    println!(
        "(b) checksum mismatch -> recovered {} frame(s)",
        chunks.len()
    );
    assert_eq!(
        chunks.len(),
        1,
        "reader stops at the first hash-mismatched frame, keeping only the valid prefix"
    );
    assert_eq!(
        &chunks[0].1, b"one_",
        "the frame before the corruption is preserved"
    );
    // No fabrication: neither the corrupted frame nor anything after it leaks.
    assert!(
        chunks.iter().all(|(_, p)| p.as_slice() != b"thre"),
        "frames after the corruption must not be fabricated into the stream"
    );
}

// ---------------------------------------------------------------------------
// (c) Corrupt DB — garbage where a SQLite database is expected.
// ---------------------------------------------------------------------------
#[test]
fn corrupt_db_surfaces_error_without_fabrication() {
    let dir = TempDir::new().expect("tmpdir");
    let db_path = dir.path().join("corrupt.db");

    // Clear garbage — NOT an empty file (an empty file is a *valid* new SQLite
    // DB). 256 bytes of 0xFF cannot be a legal SQLite page-1 header.
    std::fs::write(&db_path, vec![0xFFu8; 256]).expect("write garbage db");

    // Drive the REAL init path. `Connection::open` is lazy, so corruption may
    // surface either at open() (if a pragma forces a page-1 read) or at
    // run_migrations() (which runs CREATE TABLE + SELECT and must touch page 1).
    // Either is acceptable: both return a clean Err and neither fabricates rows.
    let open_result = SqliteStore::open(&db_path);
    println!(
        "(c) corrupt DB -> SqliteStore::open() is_err = {}",
        open_result.is_err()
    );

    match open_result {
        Err(e) => {
            // Corruption detected at open. Must be a real, descriptive error.
            let msg = e.to_string();
            println!("(c) open() error: {msg}");
            assert!(
                !msg.is_empty(),
                "open() error on a corrupt DB must carry a message"
            );
        }
        Ok(store) => {
            // open() returned lazily-Ok. The error MUST then surface at the
            // first real DB touch (migrations), before any row is served.
            let migrate_result = store.run_migrations();
            println!(
                "(c) run_migrations() on lazily-opened corrupt DB is_err = {}",
                migrate_result.is_err()
            );
            assert!(
                migrate_result.is_err(),
                "a corrupt DB must surface an error no later than run_migrations(); \
                 silently succeeding would risk fabricating an empty-but-valid store"
            );
            if let Err(e) = &migrate_result {
                let msg = e.to_string();
                println!("(c) run_migrations() error: {msg}");
                assert!(
                    !msg.is_empty(),
                    "migration error on a corrupt DB must carry a message"
                );
            }

            // Anti-fabrication: even if some method is reachable, the store must
            // not invent transcript rows out of a corrupt file. A read either
            // errors or returns an empty set — never fabricated content.
            let probe = store.list_chunks(AgentRunId(Uuid::nil()));
            println!(
                "(c) post-corruption list_chunks is_err = {}",
                probe.is_err()
            );
            match probe {
                Ok(rows) => assert!(
                    rows.is_empty(),
                    "a corrupt store must never fabricate transcript rows"
                ),
                Err(e) => {
                    // Erroring out is the preferred, non-fabricating outcome.
                    assert!(!e.to_string().is_empty(), "error must carry a message");
                }
            }
        }
    }
}
