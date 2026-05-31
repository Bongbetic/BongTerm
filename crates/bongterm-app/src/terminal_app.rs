//! Thin iced shell over the proven [`TerminalSession`] core.
//!
//! Architecture (see the slice notes in `SHIP-READINESS.md`): a background
//! thread reads ConPTY bytes into an mpsc channel; an `iced::time::every` tick
//! drains the channel into the session (parser) and refreshes the snapshot;
//! keystrokes are mapped to bytes and written back. The session lives in the
//! iced state (single-threaded), so it needs no `Send`. Rendering is a pragmatic
//! monospace text grid; the wgpu `TerminalPipeline` can swap in behind the same
//! `SurfaceSnapshot` boundary later.

use std::sync::mpsc::{Receiver, channel};

use bongterm_term::SurfaceSnapshot;
use iced::event::{self, Event};
use iced::keyboard::{self, Key, key::Named};
use iced::time::{self, Duration};
use iced::widget::container;
use iced::{Element, Length, Subscription, Task, Theme};

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
    // No explicit #[must_use]: the returned Task is already #[must_use], so the
    // tuple carries that obligation without a redundant (message-less) attribute.
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
        // Render the grid through the real wgpu/cryoglyph renderer
        // (`bongterm-render`) via Iced's shader widget, replacing the previous
        // pragmatic iced-`text` grid.
        let program = TerminalProgram {
            snapshot: to_render_snapshot(&self.snapshot),
        };
        container(
            iced::widget::shader::Shader::new(program)
                .width(Length::Fill)
                .height(Length::Fill),
        )
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
    std::env::var_os("PATH")
        .is_some_and(|paths| std::env::split_paths(&paths).any(|dir| dir.join(program).is_file()))
}

/// An Iced shader program that draws the terminal grid via `bongterm-render`.
struct TerminalProgram {
    snapshot: bongterm_render::SurfaceSnapshot,
}

impl iced::widget::shader::Program<Message> for TerminalProgram {
    type State = ();
    type Primitive = bongterm_render::TerminalPrimitive;

    fn draw(
        &self,
        _state: &Self::State,
        _cursor: iced::mouse::Cursor,
        _bounds: iced::Rectangle,
    ) -> Self::Primitive {
        // Full-surface redraw for now; dirty-region tracking is a later pass.
        bongterm_render::TerminalPrimitive::new(self.snapshot.clone(), Vec::new())
    }
}

/// Convert the terminal-core snapshot (text runs) into the renderer's
/// row-major codepoint grid. Lossy for now (no colour/attributes) — enough to
/// drive the real renderer. Multi-width glyphs advance one column (refined later).
// u32->u16 grid dims are window-bounded; truncation cannot occur in practice.
#[allow(clippy::cast_possible_truncation)]
fn to_render_snapshot(term: &SurfaceSnapshot) -> bongterm_render::SurfaceSnapshot {
    let w = term.cols as usize;
    let h = term.rows as usize;
    let mut cells = vec![0u32; w.saturating_mul(h)];
    for run in &term.runs {
        let r = run.row as usize;
        if r >= h {
            continue;
        }
        let base = run.start_col as usize;
        for (i, ch) in run.text.chars().enumerate() {
            let c = base + i;
            if c >= w {
                break;
            }
            cells[r * w + c] = ch as u32;
        }
    }
    bongterm_render::SurfaceSnapshot {
        id: bongterm_render::SnapshotId(0),
        cols: term.cols as u16,
        rows: term.rows as u16,
        cells,
    }
}

/// Map a key press to the bytes a terminal would send to the shell.
// Nested guards kept un-collapsed: the staged `Key::Character` -> first-char ->
// is-ascii checks read as a clear decision ladder; merging into let-chains would
// not change behavior but would obscure the per-step intent.
#[allow(clippy::collapsible_if)]
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
