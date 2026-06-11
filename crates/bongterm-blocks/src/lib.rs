//! `BongTerm` shell-integration command blocks.
//!
//! Parses semantic OSC sequences (OSC 133 FTCS + OSC 7 CWD) emitted by shells
//! that support shell integration, detects command block boundaries, and exposes
//! the set of user-facing actions available on each completed block.
//!
//! ## Module ownership
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.
//! bongterm-blocks owns block data and boundary detection only. Rendering,
//! input routing, and agent attachment are the responsibility of bongterm-ui
//! and bongterm-agents respectively.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

// ─── OscEvent ────────────────────────────────────────────────────────────────

/// A semantic event parsed from a shell-integration OSC sequence.
///
/// Input is the raw OSC *payload* — the bytes between `ESC ]` and the string
/// terminator (`ST` = `ESC \` or `BEL`). The framing bytes are not included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OscEvent {
    /// `OSC 133;A` — shell is about to display the prompt.
    PromptStart,
    /// `OSC 133;B` — prompt finished; cursor is at the command-input position.
    CommandStart,
    /// `OSC 133;C` — user pressed Enter; command output begins.
    CommandExecuted,
    /// `OSC 133;D;<exit_code>` — command finished with `exit_code`.
    CommandFinished(i32),
    /// `OSC 7;<uri>` — shell reported a new working directory URI.
    WorkingDir(String),
    /// Sequence not recognized by `BongTerm` shell integration.
    Unrecognized,
}

/// Parse a single OSC payload into an [`OscEvent`].
///
/// `payload` is the raw bytes between `ESC ]` and `ST`/`BEL` — caller strips
/// the framing before calling this function.
#[must_use]
pub fn parse_osc(payload: &[u8]) -> OscEvent {
    // Split on the first ';' to get the numeric code.
    let (code_bytes, rest) = match payload.iter().position(|&b| b == b';') {
        Some(pos) => (&payload[..pos], &payload[pos + 1..]),
        None => (payload, &[] as &[u8]),
    };

    match std::str::from_utf8(code_bytes).unwrap_or("").trim() {
        "133" => parse_ftcs(rest),
        "7" => {
            let uri = std::str::from_utf8(rest).unwrap_or("").to_string();
            if uri.is_empty() {
                OscEvent::Unrecognized
            } else {
                OscEvent::WorkingDir(uri)
            }
        }
        _ => OscEvent::Unrecognized,
    }
}

fn parse_ftcs(payload: &[u8]) -> OscEvent {
    match payload {
        b"A" => OscEvent::PromptStart,
        b"B" => OscEvent::CommandStart,
        b"C" => OscEvent::CommandExecuted,
        _ if payload.starts_with(b"D;") => {
            let code_str = std::str::from_utf8(&payload[2..]).unwrap_or("0");
            let code: i32 = code_str.trim().parse().unwrap_or(0);
            OscEvent::CommandFinished(code)
        }
        _ => OscEvent::Unrecognized,
    }
}

// ─── Confidence ──────────────────────────────────────────────────────────────

/// Reliability grade for shell integration in the current session.
///
/// Ordered: `Unsupported < Low < Medium < High`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// No shell-integration OSC events seen.
    Unsupported,
    /// Only `OSC 133;D` (command finished) seen — exit codes available,
    /// but prompt boundaries are unknown.
    Low,
    /// `OSC 133;A` (prompt start) and `OSC 133;D` seen, but not B/C.
    /// Block boundaries are approximate.
    Medium,
    /// All four FTCS markers (A/B/C/D) seen — full block boundaries available.
    High,
}

// ─── CommandBlock ─────────────────────────────────────────────────────────────

/// A single, completed command block detected by shell integration.
///
/// `command` is empty until command-text capture is wired (requires
/// intercepting raw key input in the PTY layer — deferred to a later task).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandBlock {
    /// Raw command text typed by the user. Empty until PTY input capture is wired.
    pub command: String,
    /// Exit code from `OSC 133;D;<code>`. `None` if the block closed without D.
    pub exit_code: Option<i32>,
    /// Working directory at the time this block was closed (from `OSC 7`).
    pub cwd: Option<String>,
}

// ─── BlockBuilder ────────────────────────────────────────────────────────────

/// Stateful consumer of [`OscEvent`]s that detects command block boundaries.
///
/// Feed events with [`BlockBuilder::push`]; a completed [`CommandBlock`] is
/// returned whenever a block boundary is detected. Query session-level
/// [`Confidence`] with [`BlockBuilder::confidence`].
///
/// Create one `BlockBuilder` per pane; reset between sessions with
/// [`BlockBuilder::reset`].
// Each bool is an independent, named state/confidence flag with distinct
// lifetime semantics (one transient, four session-level). Packing them into a
// single bitfield would obscure intent without changing behavior.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default)]
pub struct BlockBuilder {
    current_cwd: Option<String>,
    in_command: bool,
    // Confidence flags (session-level, never cleared).
    seen_prompt_start: bool,
    seen_command_start: bool,
    seen_command_executed: bool,
    seen_command_finished: bool,
}

impl BlockBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset transient state (current command in flight) without clearing
    /// session-level confidence flags.
    pub fn reset(&mut self) {
        self.in_command = false;
    }

    /// Feed an [`OscEvent`]. Returns a completed [`CommandBlock`] at each
    /// block boundary (`OSC 133;D` closes a block).
    #[must_use]
    pub fn push(&mut self, event: OscEvent) -> Option<CommandBlock> {
        match event {
            OscEvent::PromptStart => {
                self.seen_prompt_start = true;
                // A new prompt means any unfinished command is abandoned.
                self.in_command = false;
                None
            }
            OscEvent::CommandStart => {
                self.seen_command_start = true;
                None
            }
            OscEvent::CommandExecuted => {
                self.seen_command_executed = true;
                self.in_command = true;
                None
            }
            OscEvent::CommandFinished(code) => {
                self.seen_command_finished = true;
                self.in_command = false;
                // Emit a block whenever D arrives, regardless of prior markers.
                Some(CommandBlock {
                    command: String::new(), // TODO(1.E): wire PTY input capture
                    exit_code: Some(code),
                    cwd: self.current_cwd.clone(),
                })
            }
            OscEvent::WorkingDir(uri) => {
                self.current_cwd = Some(uri);
                None
            }
            OscEvent::Unrecognized => None,
        }
    }

    /// Session-level confidence grade based on which FTCS markers have been seen.
    #[must_use]
    pub fn confidence(&self) -> Confidence {
        match (
            self.seen_prompt_start,
            self.seen_command_start,
            self.seen_command_executed,
            self.seen_command_finished,
        ) {
            (true, true, true, true) => Confidence::High,
            // A + D (or C + D) without full set → Medium
            (true, _, _, true) | (_, _, true, true) => Confidence::Medium,
            // Only D → Low
            (false, false, false, true) => Confidence::Low,
            _ => Confidence::Unsupported,
        }
    }
}

// ─── BlockAction ─────────────────────────────────────────────────────────────

/// User-facing action that can be performed on a completed [`CommandBlock`].
///
/// The actual binding (clipboard, PTY write, agent IPC) is the responsibility
/// of `bongterm-ui` and `bongterm-agents`. `bongterm-blocks` only declares
/// which actions are structurally available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlockAction {
    /// Copy command text to the clipboard.
    Copy,
    /// Re-send command text to the active pane.
    ///
    /// Only available when the block has non-empty command text.
    Rerun,
    /// Attach this block's context to the current agent.
    Attach,
    /// Save command text as a named snippet.
    ///
    /// Only available when the block has non-empty command text.
    SaveSnippet,
}

/// Return the [`BlockAction`]s structurally available for `block`.
///
/// `Copy` and `Attach` are always present (even for blocks with no command
/// text, since the output is useful). `Rerun` and `SaveSnippet` require
/// non-empty command text.
#[must_use]
pub fn available_actions(block: &CommandBlock) -> Vec<BlockAction> {
    let mut actions = vec![BlockAction::Copy, BlockAction::Attach];
    if !block.command.is_empty() {
        actions.push(BlockAction::Rerun);
        actions.push(BlockAction::SaveSnippet);
    }
    actions
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_osc ───────────────────────────────────────────────────────────

    #[test]
    fn parse_prompt_start() {
        assert_eq!(parse_osc(b"133;A"), OscEvent::PromptStart);
    }

    #[test]
    fn parse_command_start() {
        assert_eq!(parse_osc(b"133;B"), OscEvent::CommandStart);
    }

    #[test]
    fn parse_command_executed() {
        assert_eq!(parse_osc(b"133;C"), OscEvent::CommandExecuted);
    }

    #[test]
    fn parse_command_finished_zero() {
        assert_eq!(parse_osc(b"133;D;0"), OscEvent::CommandFinished(0));
    }

    #[test]
    fn parse_command_finished_nonzero() {
        assert_eq!(parse_osc(b"133;D;127"), OscEvent::CommandFinished(127));
    }

    #[test]
    fn parse_command_finished_negative() {
        assert_eq!(parse_osc(b"133;D;-1"), OscEvent::CommandFinished(-1));
    }

    #[test]
    fn parse_working_dir() {
        assert_eq!(
            parse_osc(b"7;file:///c:/projects/bongt"),
            OscEvent::WorkingDir("file:///c:/projects/bongt".to_string())
        );
    }

    #[test]
    fn parse_unknown_code_is_unrecognized() {
        assert_eq!(parse_osc(b"999;something"), OscEvent::Unrecognized);
    }

    #[test]
    fn parse_empty_payload_is_unrecognized() {
        assert_eq!(parse_osc(b""), OscEvent::Unrecognized);
    }

    #[test]
    fn parse_133_unknown_marker_is_unrecognized() {
        assert_eq!(parse_osc(b"133;Z"), OscEvent::Unrecognized);
    }

    #[test]
    fn parse_osc7_empty_uri_is_unrecognized() {
        assert_eq!(parse_osc(b"7;"), OscEvent::Unrecognized);
    }

    // ── Confidence ──────────────────────────────────────────────────────────

    #[test]
    fn confidence_unsupported_initially() {
        let b = BlockBuilder::new();
        assert_eq!(b.confidence(), Confidence::Unsupported);
    }

    #[test]
    fn confidence_low_after_only_d() {
        let mut b = BlockBuilder::new();
        let _ = b.push(OscEvent::CommandFinished(0));
        assert_eq!(b.confidence(), Confidence::Low);
    }

    #[test]
    fn confidence_medium_after_a_and_d() {
        let mut b = BlockBuilder::new();
        let _ = b.push(OscEvent::PromptStart);
        let _ = b.push(OscEvent::CommandFinished(0));
        assert_eq!(b.confidence(), Confidence::Medium);
    }

    #[test]
    fn confidence_medium_after_c_and_d() {
        let mut b = BlockBuilder::new();
        let _ = b.push(OscEvent::CommandExecuted);
        let _ = b.push(OscEvent::CommandFinished(0));
        assert_eq!(b.confidence(), Confidence::Medium);
    }

    #[test]
    fn confidence_high_after_all_four_markers() {
        let mut b = BlockBuilder::new();
        let _ = b.push(OscEvent::PromptStart);
        let _ = b.push(OscEvent::CommandStart);
        let _ = b.push(OscEvent::CommandExecuted);
        let _ = b.push(OscEvent::CommandFinished(0));
        assert_eq!(b.confidence(), Confidence::High);
    }

    #[test]
    fn confidence_ordering_low_lt_medium_lt_high() {
        assert!(Confidence::Low < Confidence::Medium);
        assert!(Confidence::Medium < Confidence::High);
        assert!(Confidence::Unsupported < Confidence::Low);
    }

    // ── BlockBuilder ────────────────────────────────────────────────────────

    #[test]
    fn full_sequence_emits_one_block_on_d() {
        let mut b = BlockBuilder::new();
        assert!(b.push(OscEvent::PromptStart).is_none());
        assert!(b.push(OscEvent::CommandStart).is_none());
        assert!(b.push(OscEvent::CommandExecuted).is_none());
        let block = b.push(OscEvent::CommandFinished(0)).unwrap();
        assert_eq!(block.exit_code, Some(0));
    }

    #[test]
    fn exit_only_sequence_still_emits_block() {
        let mut b = BlockBuilder::new();
        let block = b.push(OscEvent::CommandFinished(42)).unwrap();
        assert_eq!(block.exit_code, Some(42));
    }

    #[test]
    fn cwd_before_d_is_included_in_block() {
        let mut b = BlockBuilder::new();
        let _ = b.push(OscEvent::WorkingDir("/home/user".to_string()));
        let block = b.push(OscEvent::CommandFinished(0)).unwrap();
        assert_eq!(block.cwd, Some("/home/user".to_string()));
    }

    #[test]
    fn cwd_after_d_is_not_in_previous_block() {
        let mut b = BlockBuilder::new();
        let block = b.push(OscEvent::CommandFinished(0)).unwrap();
        let _ = b.push(OscEvent::WorkingDir("/new/dir".to_string()));
        assert_eq!(block.cwd, None);
    }

    #[test]
    fn prompt_start_abandons_in_flight_command() {
        let mut b = BlockBuilder::new();
        let _ = b.push(OscEvent::CommandExecuted); // in_command = true
        let _ = b.push(OscEvent::PromptStart); // should clear in_command
        // Another command cycle
        let _ = b.push(OscEvent::CommandExecuted);
        let block = b.push(OscEvent::CommandFinished(0)).unwrap();
        assert_eq!(block.exit_code, Some(0));
    }

    #[test]
    fn nonzero_exit_code_preserved_in_block() {
        let mut b = BlockBuilder::new();
        let block = b.push(OscEvent::CommandFinished(127)).unwrap();
        assert_eq!(block.exit_code, Some(127));
    }

    #[test]
    fn unrecognized_events_ignored() {
        let mut b = BlockBuilder::new();
        assert!(b.push(OscEvent::Unrecognized).is_none());
        assert_eq!(b.confidence(), Confidence::Unsupported);
    }

    // ── Fixture-based tests ─────────────────────────────────────────────────

    fn replay_fixture(name: &str) -> (Vec<CommandBlock>, Confidence) {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("tests/fixtures/osc")
            .join(name);
        let content =
            std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {name}: {e}"));
        let mut builder = BlockBuilder::new();
        let mut blocks = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(block) = builder.push(parse_osc(line.as_bytes())) {
                blocks.push(block);
            }
        }
        (blocks, builder.confidence())
    }

    #[test]
    fn fixture_bash_session_high_confidence_one_block() {
        let (blocks, confidence) = replay_fixture("bash_session.txt");
        assert_eq!(confidence, Confidence::High);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].exit_code, Some(0));
        assert_eq!(blocks[0].cwd.as_deref(), Some("file:///c:/projects/bongt"));
    }

    #[test]
    fn fixture_powershell_session_high_confidence_exit_one() {
        let (blocks, confidence) = replay_fixture("powershell_session.txt");
        assert_eq!(confidence, Confidence::High);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].exit_code, Some(1));
    }

    #[test]
    fn fixture_partial_osc_medium_confidence() {
        let (blocks, confidence) = replay_fixture("partial_osc.txt");
        assert_eq!(confidence, Confidence::Medium);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn fixture_exit_only_low_confidence() {
        let (blocks, confidence) = replay_fixture("exit_only.txt");
        assert_eq!(confidence, Confidence::Low);
        assert_eq!(blocks.len(), 1);
    }

    #[test]
    fn fixture_no_osc_unsupported_no_blocks() {
        let (blocks, confidence) = replay_fixture("no_osc.txt");
        assert_eq!(confidence, Confidence::Unsupported);
        assert!(blocks.is_empty());
    }

    // ── BlockAction ─────────────────────────────────────────────────────────

    #[test]
    fn available_actions_empty_command_copy_and_attach_only() {
        let block = CommandBlock {
            command: String::new(),
            exit_code: Some(0),
            cwd: None,
        };
        let actions = available_actions(&block);
        assert!(actions.contains(&BlockAction::Copy));
        assert!(actions.contains(&BlockAction::Attach));
        assert!(!actions.contains(&BlockAction::Rerun));
        assert!(!actions.contains(&BlockAction::SaveSnippet));
    }

    #[test]
    fn available_actions_with_command_text_includes_rerun_and_save() {
        let block = CommandBlock {
            command: "cargo test".to_string(),
            exit_code: Some(0),
            cwd: None,
        };
        let actions = available_actions(&block);
        assert!(actions.contains(&BlockAction::Rerun));
        assert!(actions.contains(&BlockAction::SaveSnippet));
    }

    #[test]
    fn available_actions_always_includes_copy() {
        let block = CommandBlock {
            command: String::new(),
            exit_code: None,
            cwd: None,
        };
        assert!(available_actions(&block).contains(&BlockAction::Copy));
    }
}
