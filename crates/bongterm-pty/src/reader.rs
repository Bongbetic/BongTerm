//! PTY reader task: drains the PTY master read pipe into [`Slab`] buffers.
//!
//! A bounded channel applies backpressure: when the downstream consumer
//! (parser, transcript writer) is slow, the read loop blocks rather than
//! growing an unbounded queue.

use crate::ring::{Slab, SlabPool};
use std::io::Read;
use std::sync::mpsc::{self, Receiver};
use std::thread;

/// A background thread that drains a PTY reader into [`Slab`] buffers.
///
/// The outbound channel is bounded (`capacity`): when the consumer is slow
/// the sender blocks, applying backpressure to the read loop.
///
/// The channel closes when the PTY pipe closes (EOF) or a read error occurs.
pub struct PtyReaderTask {
    /// Receive filled slabs. Channel closes when the PTY pipe closes.
    pub rx: Receiver<Slab>,
    _handle: thread::JoinHandle<()>,
}

impl PtyReaderTask {
    /// Spawn the reader thread.
    ///
    /// - `reader`: read half from [`PtyChild::take_reader()`].
    /// - `pool`: slab pool; one slab per read, returned to pool on drop.
    /// - `capacity`: bound on the outbound channel.
    #[must_use]
    pub fn spawn(mut reader: Box<dyn Read + Send>, pool: SlabPool, capacity: usize) -> Self {
        let (tx, rx) = mpsc::sync_channel(capacity);
        let handle = thread::spawn(move || {
            loop {
                let mut slab = pool.acquire();
                match reader.read(slab.buf_mut()) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        slab.set_used(n);
                        if tx.send(slab).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Self {
            rx,
            _handle: handle,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};

    #[test]
    fn reader_task_delivers_written_bytes() {
        let data = b"hello pty world";
        let cursor = Cursor::new(data.to_vec());
        let pool = SlabPool::new(2);
        let task = PtyReaderTask::spawn(Box::new(cursor), pool, 4);

        let slab = task.rx.recv().expect("should receive slab with data");
        assert_eq!(slab.slice(), data.as_ref());
    }

    #[test]
    fn reader_task_channel_closes_on_eof() {
        let cursor = Cursor::new(b"x".to_vec());
        let pool = SlabPool::new(2);
        let task = PtyReaderTask::spawn(Box::new(cursor), pool, 4);

        task.rx.recv().expect("should receive first slab");
        assert!(task.rx.recv().is_err(), "channel should close after EOF");
    }

    #[test]
    fn reader_task_no_data_loss_under_backpressure() {
        let data: Vec<u8> = (0u64..8192 * 8)
            .map(|i| u8::try_from(i % 256).unwrap())
            .collect();
        let cursor = Cursor::new(data.clone());
        let pool = SlabPool::new(4);
        let task = PtyReaderTask::spawn(Box::new(cursor), pool, 2); // capacity=2 << 8 slabs

        let mut received = Vec::new();
        while let Ok(slab) = task.rx.recv() {
            received.extend_from_slice(slab.slice());
        }
        assert_eq!(received, data, "no bytes lost under backpressure");
    }

    #[test]
    fn reader_task_channel_closes_on_io_error() {
        struct FailAfterOnce {
            data: Option<Vec<u8>>,
        }
        impl Read for FailAfterOnce {
            fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
                if let Some(data) = self.data.take() {
                    let n = data.len().min(buf.len());
                    buf[..n].copy_from_slice(&data[..n]);
                    Ok(n)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "pipe broken",
                    ))
                }
            }
        }

        let pool = SlabPool::new(2);
        let reader = FailAfterOnce {
            data: Some(b"ok".to_vec()),
        };
        let task = PtyReaderTask::spawn(Box::new(reader), pool, 4);

        let slab = task.rx.recv().expect("should receive pre-error data");
        assert_eq!(slab.slice(), b"ok");
        assert!(
            task.rx.recv().is_err(),
            "channel should close after IO error"
        );
    }
}
