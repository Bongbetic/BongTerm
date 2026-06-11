//! PTY dispatcher: fans a single `PtyReaderTask` byte stream out to two
//! bounded consumers (typically the renderer path and the transcript path).
//!
//! Backpressure from either consumer propagates back to the PTY read loop.
//! Slabs are `Arc`-wrapped so the backing buffer is not returned to the pool
//! until **both** consumers have released their reference.

use crate::ring::Slab;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// Fans out a [`PtyReaderTask`] byte stream to two downstream consumers.
pub struct PtyDispatcher {
    /// Channel A — renderer path.
    pub rx_a: Receiver<Arc<Slab>>,
    /// Channel B — transcript-writer path.
    pub rx_b: Receiver<Arc<Slab>>,
    _handle: thread::JoinHandle<()>,
}

impl PtyDispatcher {
    /// Spawn the dispatcher thread.
    ///
    /// - `source`: outbound channel from `PtyReaderTask`.
    /// - `capacity_a` / `capacity_b`: channel bounds.
    #[must_use]
    pub fn spawn(source: Receiver<Slab>, capacity_a: usize, capacity_b: usize) -> Self {
        let (tx_a, rx_a) = mpsc::sync_channel::<Arc<Slab>>(capacity_a);
        let (tx_b, rx_b) = mpsc::sync_channel::<Arc<Slab>>(capacity_b);
        let handle = thread::spawn(move || {
            while let Ok(slab) = source.recv() {
                let arc = Arc::new(slab);
                if tx_a.send(Arc::clone(&arc)).is_err() {
                    break;
                }
                if tx_b.send(arc).is_err() {
                    break;
                }
            }
        });
        Self {
            rx_a,
            rx_b,
            _handle: handle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::PtyReaderTask;
    use crate::ring::SlabPool;
    use std::io::Cursor;

    #[test]
    fn dispatcher_delivers_to_both_consumers() {
        let data = b"hello world";
        let cursor = Cursor::new(data.to_vec());
        let pool = SlabPool::new(2);
        let reader = PtyReaderTask::spawn(Box::new(cursor), pool, 4);
        let dispatcher = PtyDispatcher::spawn(reader.rx, 4, 4);

        let slab_a = dispatcher.rx_a.recv().expect("rx_a should receive slab");
        let slab_b = dispatcher.rx_b.recv().expect("rx_b should receive slab");
        assert_eq!(slab_a.slice(), data.as_ref());
        assert_eq!(slab_b.slice(), data.as_ref());
    }

    #[test]
    fn dispatcher_slow_renderer_no_byte_loss() {
        // rx_a capacity=1 simulates heavy backpressure on the renderer path.
        let data: Vec<u8> = (0u64..8192 * 4)
            .map(|i| u8::try_from(i % 256).unwrap())
            .collect();
        let cursor = Cursor::new(data.clone());
        let pool = SlabPool::new(4);
        let reader = PtyReaderTask::spawn(Box::new(cursor), pool, 2);
        let dispatcher = PtyDispatcher::spawn(reader.rx, 1, 4);

        // Fast transcript consumer.
        let transcript_thread = thread::spawn(move || {
            let mut received = Vec::new();
            while let Ok(slab) = dispatcher.rx_b.recv() {
                received.extend_from_slice(slab.slice());
            }
            received
        });

        // Slow renderer consumer (this thread, small channel forces blocking).
        let mut renderer_received = Vec::new();
        while let Ok(slab) = dispatcher.rx_a.recv() {
            renderer_received.extend_from_slice(slab.slice());
        }

        let transcript_received = transcript_thread
            .join()
            .expect("transcript thread panicked");
        assert_eq!(
            renderer_received, data,
            "renderer lost bytes under backpressure"
        );
        assert_eq!(
            transcript_received, data,
            "transcript lost bytes under backpressure"
        );
    }

    #[test]
    fn dispatcher_slow_transcript_no_byte_loss() {
        // rx_b capacity=1 simulates heavy backpressure on the transcript path.
        let data: Vec<u8> = (0u64..8192 * 4)
            .map(|i| u8::try_from(i % 256).unwrap())
            .collect();
        let cursor = Cursor::new(data.clone());
        let pool = SlabPool::new(4);
        let reader = PtyReaderTask::spawn(Box::new(cursor), pool, 2);
        let dispatcher = PtyDispatcher::spawn(reader.rx, 4, 1);

        // Fast renderer consumer in separate thread.
        let renderer_thread = thread::spawn(move || {
            let mut received = Vec::new();
            while let Ok(slab) = dispatcher.rx_a.recv() {
                received.extend_from_slice(slab.slice());
            }
            received
        });

        // Slow transcript consumer (this thread, small channel forces blocking).
        let mut transcript_received = Vec::new();
        while let Ok(slab) = dispatcher.rx_b.recv() {
            transcript_received.extend_from_slice(slab.slice());
        }

        let renderer_received = renderer_thread.join().expect("renderer thread panicked");
        assert_eq!(
            renderer_received, data,
            "renderer lost bytes under backpressure"
        );
        assert_eq!(
            transcript_received, data,
            "transcript lost bytes under backpressure"
        );
    }

    #[test]
    fn dispatcher_both_channels_close_on_pty_eof() {
        let cursor = Cursor::new(b"x".to_vec());
        let pool = SlabPool::new(2);
        let reader = PtyReaderTask::spawn(Box::new(cursor), pool, 4);
        let dispatcher = PtyDispatcher::spawn(reader.rx, 4, 4);

        dispatcher
            .rx_a
            .recv()
            .expect("rx_a should receive first slab");
        dispatcher
            .rx_b
            .recv()
            .expect("rx_b should receive first slab");
        assert!(
            dispatcher.rx_a.recv().is_err(),
            "rx_a should close after PTY EOF"
        );
        assert!(
            dispatcher.rx_b.recv().is_err(),
            "rx_b should close after PTY EOF"
        );
    }
}
