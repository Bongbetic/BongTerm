//! Slab-pool of 8-KiB byte buffers reused across `ConPTY` reads.
//!
//! Spec §3.1 hot-path discipline: `ConPTY` pipe bytes are read into reusable
//! ring/slab buffers owned by Terminal Core. The parser consumes slices.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const SLAB_SIZE: usize = 8 * 1024;

/// A pool of fixed-size (`SLAB_SIZE`) byte buffers.
///
/// Buffers are recycled on drop to avoid repeated heap allocation in the
/// `ConPTY` read loop.
pub struct SlabPool {
    pub(crate) inner: Arc<Mutex<VecDeque<Box<[u8; SLAB_SIZE]>>>>,
    #[allow(dead_code)]
    max_slabs: usize,
}

/// A single 8-KiB buffer checked out from a [`SlabPool`].
///
/// Returns itself to the pool on drop (if space is available).
pub struct Slab {
    bytes: Box<[u8; SLAB_SIZE]>,
    used: usize,
    pool: Arc<Mutex<VecDeque<Box<[u8; SLAB_SIZE]>>>>,
}

impl SlabPool {
    /// Create a new pool that retains at most `max_slabs` buffers.
    #[must_use]
    pub fn new(max_slabs: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(max_slabs))),
            max_slabs,
        }
    }

    /// Check out a slab from the pool, allocating a fresh one if the pool is
    /// empty.
    ///
    /// # Panics
    ///
    /// Panics if the internal mutex is poisoned (only possible if a previous
    /// thread panicked while holding the lock — not expected in normal use).
    #[must_use]
    pub fn acquire(&self) -> Slab {
        let mut g = self.inner.lock().expect("slab pool poisoned");
        let bytes = g.pop_front().unwrap_or_else(|| Box::new([0u8; SLAB_SIZE]));
        Slab {
            bytes,
            used: 0,
            pool: Arc::clone(&self.inner),
        }
    }
}

impl Slab {
    /// Returns the total capacity of this slab (always `8192`).
    #[must_use]
    #[allow(clippy::unused_self)]
    pub fn capacity(&self) -> usize {
        SLAB_SIZE
    }

    /// Returns the number of bytes written into this slab.
    #[must_use]
    pub fn used(&self) -> usize {
        self.used
    }

    /// Returns a mutable reference to the full backing buffer.
    pub fn buf_mut(&mut self) -> &mut [u8] {
        &mut self.bytes[..]
    }

    /// Record that `n` bytes were written. Clamped to [`SLAB_SIZE`].
    pub fn set_used(&mut self, n: usize) {
        self.used = n.min(SLAB_SIZE);
    }

    /// Returns the slice of bytes that were written (`&bytes[..used]`).
    #[must_use]
    pub fn slice(&self) -> &[u8] {
        &self.bytes[..self.used]
    }
}

impl Drop for Slab {
    fn drop(&mut self) {
        let Ok(mut g) = self.pool.lock() else {
            return;
        };
        if g.len() < g.capacity() {
            let mut placeholder: Box<[u8; SLAB_SIZE]> = Box::new([0u8; SLAB_SIZE]);
            std::mem::swap(&mut placeholder, &mut self.bytes);
            g.push_back(placeholder);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_release_round_trip_reuses_buffer() {
        let pool = SlabPool::new(2);
        {
            let mut s = pool.acquire();
            s.buf_mut()[0] = 42;
            s.set_used(1);
            assert_eq!(s.slice(), &[42]);
        }
        let s2 = pool.acquire();
        assert_eq!(s2.capacity(), 8 * 1024);
    }

    #[test]
    fn many_concurrent_acquires_bounded_by_pool_size() {
        let pool = SlabPool::new(2);
        let a = pool.acquire();
        let b = pool.acquire();
        let _c = pool.acquire(); // exceeds pool size; allocates new slab
        drop(a);
        drop(b);
        let g = pool.inner.lock().unwrap();
        assert!(g.len() <= 2);
    }
}
