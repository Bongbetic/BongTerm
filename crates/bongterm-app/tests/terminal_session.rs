//! Headless proof that the vertical slice works end-to-end:
//! spawn a real shell → write input → read ConPTY bytes → VT-parse → snapshot.
//! No GUI required; this test IS the evidence that the terminal runs.

use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

use bongterm_app::session::TerminalSession;

/// Spawn `cmd.exe`, type `echo hello` + `exit`, and assert the parsed grid
/// snapshot contains the echoed text. Proves spawn → write → read → parse.
#[test]
fn shell_echo_round_trips_into_snapshot() {
    let (mut session, reader) =
        TerminalSession::spawn_command("cmd.exe", &[], 80, 24).expect("spawn cmd.exe");

    session
        .write_input(b"echo hello\r\nexit\r\n")
        .expect("write input to shell");

    // Read on a worker thread (the reader is Send; the session is not required
    // to be). Bounded so a misbehaving shell fails the test instead of hanging.
    let (tx, rx) = mpsc::channel::<Vec<u8>>();
    let pump = std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match std::io::Read::read(&mut reader, &mut buf) {
                Ok(0) | Err(_) => break, // EOF (child exited) or pipe closed
                Ok(n) => {
                    if tx.send(buf[..n].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });

    // ConPTY does not reliably signal EOF while the master is open, so read
    // until the output goes idle after some has arrived (or EOF, or a hard cap).
    let start = Instant::now();
    let mut last_data: Option<Instant> = None;
    loop {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(chunk) => {
                session.feed(&chunk);
                last_data = Some(Instant::now());
            }
            Err(RecvTimeoutError::Disconnected) => break, // reader hit EOF
            Err(RecvTimeoutError::Timeout) => {
                let idle = last_data.is_some_and(|t| t.elapsed() > Duration::from_millis(800));
                if idle || start.elapsed() > Duration::from_secs(15) {
                    break;
                }
            }
        }
    }

    let text = session.snapshot_text();
    // Closing the master lets the (otherwise EOF-less) reader thread exit; detach
    // rather than join so a stuck read can never hang the test.
    drop(session);
    drop(pump);

    assert!(
        text.contains("hello"),
        "parsed grid snapshot must contain the echoed 'hello'; got:\n{text}"
    );
}
