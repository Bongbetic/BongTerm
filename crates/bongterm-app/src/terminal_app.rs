//! Thin iced shell over an event-driven ConPTY worker.
//!
//! Architecture: a per-pane `Subscription` worker (see [`pane_worker`]) owns the
//! ConPTY child and a blocking reader thread, and emits `Message::Output` *only*
//! when bytes actually arrive — so an idle shell produces no messages, no
//! repaints, and ~zero idle CPU (gate #6, vs the previous unconditional 33 ms
//! tick). Keystrokes flow the other way through a `tokio` channel handed to the
//! app via `Message::Ready`. The VT parser/grid (`bongterm-term`) lives in the
//! app state (it need not be `Send`); the renderer draws the latest snapshot via
//! Iced's wgpu shader widget.
//!
//! The worker is keyed by shell via `Subscription::run_with`, so Phase-1 #7
//! (split panes) becomes "instantiate one worker per pane id + route input",
//! not a rewrite.

use bongterm_pty::{ChildSpec, PortablePtyHost, PtyHost};
use bongterm_term::WezTermAdapter;
use iced::event::{self, Event};
use iced::futures::{SinkExt, Stream};
use iced::keyboard::{self, Key, key::Named};
use iced::widget::container;
use iced::{Element, Length, Subscription, Task, Theme};

/// Initial terminal geometry, used until the first window-resize event arrives.
const COLS: u16 = 80;
const ROWS: u16 = 24;
/// Font size (logical px) — must match `bongterm-render`'s `prepare`.
const FONT_SIZE: f32 = 14.0;
/// Container padding (logical px, per side) applied in `view`.
const PADDING: f32 = 8.0;

pub struct TerminalApp {
    /// VT parser + grid. Fed by `Message::Output`; lives on the UI thread, so it
    /// need not be `Send` (the PTY I/O that does cross threads lives in the worker).
    adapter: WezTermAdapter,
    /// Latest render snapshot for `view` (already converted from the term grid).
    snapshot: bongterm_render::SurfaceSnapshot,
    /// Channel to the PTY worker for input + resize; arrives via `Message::Ready`.
    input: Option<tokio::sync::mpsc::Sender<WorkerCmd>>,
    /// Cached cell size (logical px) for mapping window size → cols/rows.
    cell_w: f32,
    cell_h: f32,
    /// Current grid dimensions; guards against redundant resizes.
    cols: u16,
    rows: u16,
}

/// A command from the app to a pane's PTY worker.
#[derive(Debug, Clone)]
pub enum WorkerCmd {
    /// Bytes to write to the child (a keystroke).
    Input(Vec<u8>),
    /// New terminal dimensions (after a window resize).
    Resize { cols: u16, rows: u16 },
}

#[derive(Debug, Clone)]
pub enum Message {
    /// The worker is live; carries the channel for input + resize commands.
    Ready(tokio::sync::mpsc::Sender<WorkerCmd>),
    /// Raw bytes read from the ConPTY child.
    Output(Vec<u8>),
    /// Bytes to write to the child (mapped from a keystroke).
    Input(Vec<u8>),
    /// Window resized to the given logical `(width, height)`.
    Resized(f32, f32),
}

impl TerminalApp {
    // No explicit #[must_use]: the returned Task is already #[must_use], so the
    // tuple carries that obligation without a redundant (message-less) attribute.
    pub fn boot() -> (Self, Task<Message>) {
        // The parser/grid starts empty; the worker (subscription) spawns the PTY
        // and the first prompt arrives as a `Message::Output`.
        let (cell_w, cell_h) = bongterm_render::startup_monospace_cell_size(FONT_SIZE);
        let mut adapter = WezTermAdapter::new(u32::from(COLS), u32::from(ROWS));
        let snapshot = to_render_snapshot(&adapter.current_snapshot());
        (
            Self {
                adapter,
                snapshot,
                input: None,
                cell_w,
                cell_h,
                cols: COLS,
                rows: ROWS,
            },
            Task::none(),
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Ready(input) => self.input = Some(input),
            Message::Output(bytes) => {
                self.adapter.ingest_bytes(&bytes);
                self.snapshot = to_render_snapshot(&self.adapter.current_snapshot());
            }
            Message::Input(bytes) => {
                if let Some(tx) = &self.input {
                    // Keystrokes are tiny and infrequent; on the rare full-buffer
                    // case drop rather than block the UI thread.
                    let _ = tx.try_send(WorkerCmd::Input(bytes));
                }
            }
            Message::Resized(width, height) => {
                // Map the window's content area (minus padding) to a cell grid and,
                // if it changed, reflow both the local parser and the child PTY.
                let content_w = (width - 2.0 * PADDING).max(1.0);
                let content_h = (height - 2.0 * PADDING).max(1.0);
                let (cols, rows) =
                    bongterm_render::grid_dims(content_w, content_h, self.cell_w, self.cell_h);
                if cols != self.cols || rows != self.rows {
                    self.cols = cols;
                    self.rows = rows;
                    self.adapter.resize(u32::from(cols), u32::from(rows));
                    self.snapshot = to_render_snapshot(&self.adapter.current_snapshot());
                    if let Some(tx) = &self.input {
                        let _ = tx.try_send(WorkerCmd::Resize { cols, rows });
                    }
                }
            }
        }
        Task::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // Event-driven PTY worker (no idle timer) + raw keyboard events.
        let worker = Subscription::run_with(default_shell(), pane_worker);
        let events = event::listen_raw(|raw, _status, _window| match raw {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key,
                text,
                modifiers,
                ..
            }) => key_to_bytes(&key, text.as_deref(), modifiers).map(Message::Input),
            Event::Window(iced::window::Event::Resized(size)) => {
                Some(Message::Resized(size.width, size.height))
            }
            _ => None,
        });
        Subscription::batch([worker, events])
    }

    pub fn view(&self) -> Element<'_, Message> {
        // Render the grid through the real wgpu/cryoglyph renderer
        // (`bongterm-render`) via Iced's shader widget.
        let program = TerminalProgram {
            snapshot: self.snapshot.clone(),
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
    pub fn snapshot_id(&self) -> bongterm_render::SnapshotId {
        self.snapshot.id
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

/// The event-driven PTY worker for one pane.
///
/// Owns the ConPTY child (kept alive for the worker's lifetime) plus a blocking
/// reader thread. Emits `Message::Ready` once (handing back the keystroke
/// channel), then `Message::Output` whenever the child produces bytes. An idle
/// child blocks the reader thread and parks this future — no wakeups, no frames.
///
/// Keyed by shell string so `run_with` gives a stable identity; #7 will key by
/// pane id and instantiate one worker per pane.
// `&String` is required by `Subscription::run_with`'s `fn(&D)` builder shape.
#[allow(clippy::ptr_arg)]
fn pane_worker(shell: &String) -> impl Stream<Item = Message> + use<> {
    let shell = shell.clone();
    iced::stream::channel(
        100,
        move |mut output: iced::futures::channel::mpsc::Sender<Message>| async move {
            let spec = ChildSpec {
                command: shell.into(),
                args: Vec::new(),
                cwd: std::env::current_dir().ok(),
                env: Vec::new(),
                cols: COLS,
                rows: ROWS,
            };
            let mut child = match PortablePtyHost.spawn(spec) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("bongterm-app: PTY spawn failed: {e:#}");
                    return;
                }
            };
            let Some(mut reader) = child.take_reader() else {
                eprintln!("bongterm-app: PTY reader already taken");
                return;
            };

            // App -> worker: input + resize commands.
            let (input_tx, mut input_rx) = tokio::sync::mpsc::channel::<WorkerCmd>(64);
            if output.send(Message::Ready(input_tx)).await.is_err() {
                return; // app gone
            }

            // Blocking reader thread -> async byte channel. Bounded, so a slow
            // consumer applies backpressure to the reader (and thus to ConPTY) rather
            // than growing an unbounded queue.
            let (byte_tx, mut byte_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match std::io::Read::read(&mut reader, &mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if byte_tx.blocking_send(buf[..n].to_vec()).is_err() {
                                break; // worker gone
                            }
                        }
                    }
                }
            });

            // Forward output as it arrives; write input as it arrives. Both `recv`s
            // are cancel-safe, so `select!` cannot lose a message.
            loop {
                tokio::select! {
                    maybe = byte_rx.recv() => match maybe {
                        Some(bytes) => {
                            if output.send(Message::Output(bytes)).await.is_err() {
                                break; // app gone
                            }
                        }
                        None => break, // child closed / reader ended
                    },
                    maybe = input_rx.recv() => {
                        match maybe {
                            Some(WorkerCmd::Input(bytes)) => {
                                use std::io::Write;
                                let _ = child.writer.write_all(&bytes);
                                let _ = child.writer.flush();
                            }
                            Some(WorkerCmd::Resize { cols, rows }) => {
                                let _ = child.resize(cols, rows);
                            }
                            None => break, // app dropped the command channel
                        }
                    }
                }
            }
        },
    )
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

/// Convert the terminal-core snapshot into the renderer's styled-span snapshot.
///
/// Each `bongterm-term` `CellRun` maps to a renderer `CellSpan`, carrying its
/// foreground/background colour and attribute bits (the two crates share an
/// identical attr bit layout). The cursor position and the monotonic `seq`
/// (reused as the renderer's `SnapshotId`, for later change detection) carry
/// through too. Grid dims are window-bounded, so the `u32`→`u16` conversions
/// never actually saturate.
fn to_render_snapshot(term: &bongterm_term::SurfaceSnapshot) -> bongterm_render::SurfaceSnapshot {
    let spans = term
        .runs
        .iter()
        .map(|run| bongterm_render::CellSpan {
            row: u16::try_from(run.row).unwrap_or(u16::MAX),
            col: u16::try_from(run.start_col).unwrap_or(u16::MAX),
            text: run.text.clone(),
            fg: run.fg,
            bg: run.bg,
            attrs: run.attrs,
        })
        .collect();
    bongterm_render::SurfaceSnapshot {
        id: bongterm_render::SnapshotId(term.seq),
        cols: u16::try_from(term.cols).unwrap_or(u16::MAX),
        rows: u16::try_from(term.rows).unwrap_or(u16::MAX),
        spans,
        cursor: bongterm_render::CursorVis {
            row: u16::try_from(term.cursor.position.row).unwrap_or(0),
            col: u16::try_from(term.cursor.position.col).unwrap_or(0),
            visible: term.cursor.visible,
        },
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
