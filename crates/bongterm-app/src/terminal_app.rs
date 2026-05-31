//! Thin iced shell over the proven [`TerminalSession`] core.
//!
//! Architecture (see the slice notes in `SHIP-READINESS.md`): a background
//! thread reads ConPTY bytes into an mpsc channel; an `iced::time::every` tick
//! drains the channel into the session (parser) and refreshes the snapshot;
//! keystrokes are mapped to bytes and written back. The session lives in the
//! iced state (single-threaded), so it needs no `Send`. Rendering is a pragmatic
//! monospace text grid; the wgpu `TerminalPipeline` can swap in behind the same
//! `SurfaceSnapshot` boundary later.

use std::sync::mpsc::{channel, Receiver};

use bongterm_term::SurfaceSnapshot;
use iced::event::{self, Event};
use iced::keyboard::{self, key::Named, Key};
use iced::time::{self, Duration};
use iced::widget::{column, container, text};
use iced::{Element, Font, Length, Subscription, Task, Theme};

use crate::session::TerminalSession;

/// Initial terminal geometry (v1 fixes the size; resize is deferred).
const COLS: u16 = 80;
const ROWS: u16 = 24;

pub struct TerminalApp {
    session: TerminalSession,
    output_rx: Receiver<Vec<u8>>,
    snapshot: SurfaceSnapshot,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Periodic tick: drain pending ConPTY output and refresh the snapshot.
    Tick,
    /// Bytes to write to the child (mapped from a keystroke).
    Input(Vec<u8>),
}

impl TerminalApp {
    #[must_use]
    pub fn boot() -> (Self, Task<Message>) {
        let shell = default_shell();
        let (mut session, reader) = TerminalSession::spawn_command(&shell, &[], COLS, ROWS)
            .unwrap_or_else(|e| panic!("failed to spawn shell {shell:?}: {e:#}"));

        // Background reader: blocking ConPTY reads → channel. The thread exits
        // when the master closes (app shutdown) or the pipe breaks.
        let (tx, output_rx) = channel::<Vec<u8>>();
        std::thread::spawn(move || {
            let mut reader = reader;
            let mut buf = [0u8; 8192];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let snapshot = session.snapshot();
        (
            Self {
                session,
                output_rx,
                snapshot,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                let mut changed = false;
                while let Ok(bytes) = self.output_rx.try_recv() {
                    self.session.feed(&bytes);
                    changed = true;
                }
                if changed {
                    self.snapshot = self.session.snapshot();
                }
            }
            Message::Input(bytes) => {
                let _ = self.session.write_input(&bytes);
            }
        }
        Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let ticks = time::every(Duration::from_millis(33)).map(|_| Message::Tick);
        let keys = event::listen_raw(|raw, _status, _window| {
            if let Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                text,
                modifiers,
                ..
            }) = raw
            {
                key_to_bytes(&key, text.as_deref(), modifiers).map(Message::Input)
            } else {
                None
            }
        });
        Subscription::batch([ticks, keys])
    }

    pub fn view(&self) -> Element<'_, Message> {
        let lines = render_rows(&self.snapshot);
        let rows: Vec<Element<'_, Message>> = lines
            .into_iter()
            .map(|line| {
                text(if line.is_empty() { " ".to_string() } else { line })
                    .font(Font::MONOSPACE)
                    .size(14)
                    .into()
            })
            .collect();
        container(column(rows).spacing(0))
            .padding(8)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    #[must_use]
    pub const fn theme(&self) -> Theme {
        Theme::Dark
    }

    #[must_use]
    pub fn title(&self) -> String {
        "BongTerm".to_string()
    }
}

/// Pick the default shell: `pwsh.exe` if present, else `cmd.exe`.
fn default_shell() -> String {
    if which_on_path("pwsh.exe") {
        "pwsh.exe".to_string()
    } else {
        "cmd.exe".to_string()
    }
}

fn which_on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| dir.join(program).is_file())
    })
}

/// Lay the snapshot's runs into per-row strings (column-padded).
fn render_rows(snap: &SurfaceSnapshot) -> Vec<String> {
    let mut lines = vec![String::new(); snap.rows.max(1) as usize];
    let mut runs: Vec<_> = snap.runs.iter().collect();
    runs.sort_by_key(|r| (r.row, r.start_col));
    for run in runs {
        let row = run.row as usize;
        if row >= lines.len() {
            continue;
        }
        let line = &mut lines[row];
        let start = run.start_col as usize;
        let cur = line.chars().count();
        if cur < start {
            line.push_str(&" ".repeat(start - cur));
        }
        line.push_str(&run.text);
    }
    lines
}

/// Map a key press to the bytes a terminal would send to the shell.
fn key_to_bytes(key: &Key, text: Option<&str>, modifiers: keyboard::Modifiers) -> Option<Vec<u8>> {
    if let Key::Named(named) = key {
        match named {
            Named::Enter => return Some(vec![b'\r']),
            Named::Backspace => return Some(vec![0x7f]),
            Named::Tab => return Some(vec![b'\t']),
            Named::Escape => return Some(vec![0x1b]),
            Named::ArrowUp => return Some(b"\x1b[A".to_vec()),
            Named::ArrowDown => return Some(b"\x1b[B".to_vec()),
            Named::ArrowRight => return Some(b"\x1b[C".to_vec()),
            Named::ArrowLeft => return Some(b"\x1b[D".to_vec()),
            Named::Space => return Some(vec![b' ']),
            _ => {}
        }
    }

    // Ctrl+<letter> → control byte (e.g. Ctrl+C = 0x03).
    if modifiers.control() {
        if let Key::Character(c) = key {
            if let Some(ch) = c.chars().next() {
                if ch.is_ascii_alphabetic() {
                    return Some(vec![(ch.to_ascii_lowercase() as u8) & 0x1f]);
                }
            }
        }
        return None;
    }

    // Printable text (layout/shift-aware).
    if let Some(t) = text {
        if !t.is_empty() {
            return Some(t.as_bytes().to_vec());
        }
    }
    if let Key::Character(c) = key {
        return Some(c.as_bytes().to_vec());
    }
    None
}

/// Launch the terminal window.
///
/// # Errors
/// Returns an error if the iced runtime fails to start.
pub fn run() -> iced::Result {
    iced::application(TerminalApp::boot, TerminalApp::update, TerminalApp::view)
        .title(TerminalApp::title)
        .theme(TerminalApp::theme)
        .subscription(TerminalApp::subscription)
        .run()
}
