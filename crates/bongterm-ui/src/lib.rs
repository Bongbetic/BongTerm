//! bongterm-ui
//!
//! See `docs/superpowers/specs/2026-05-27-bongt-mvp0-design.md` §1.2 for the
//! ownership matrix entry that governs what this crate may and may not own.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use bongterm_settings::{KeybindingSettings, Settings};
use iced::widget::{button, column, container, row, stack, text, text_input};
use iced::{Element, Length, Task, Theme};

pub mod accessibility;
pub mod agent_sidebar;
pub mod devux;
pub mod dpi;
pub mod ime;

pub type ShellResult = iced::Result;

// ---------------------------------------------------------------------------
// Focus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellFocus {
    Terminal,
    CommandPalette,
}

// ---------------------------------------------------------------------------
// Command availability
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAvailability {
    Active,
    DisabledUntilPhase3,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    // Phase 1 active
    OpenCommandPalette,
    ReloadSettings,
    NewTab,
    ClosePane,
    SplitPane,
    FindInPane,
    OpenResourceDashboard,
    // Phase 3 stubs — shown disabled in Phase 1
    CmdK,
    SmartHistory,
    ExplainLastFailed,
    AttachContext,
    ToggleBackgroundJobs,
}

#[derive(Debug, Clone)]
pub struct CommandDefinition {
    pub id: CommandId,
    pub title: &'static str,
    pub category: &'static str,
    pub aliases: &'static [&'static str],
    pub availability: CommandAvailability,
}

impl CommandDefinition {
    fn matches(&self, query: &str) -> bool {
        self.title.to_ascii_lowercase().contains(query)
            || self.category.to_ascii_lowercase().contains(query)
            || self
                .aliases
                .iter()
                .any(|alias| alias.to_ascii_lowercase().contains(query))
    }
}

// ---------------------------------------------------------------------------
// Command palette
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CommandPalette {
    commands: Vec<CommandDefinition>,
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self {
            commands: vec![
                CommandDefinition {
                    id: CommandId::OpenCommandPalette,
                    title: "Open Command Palette",
                    category: "Shell",
                    aliases: &["palette", "commands"],
                    availability: CommandAvailability::Active,
                },
                CommandDefinition {
                    id: CommandId::ReloadSettings,
                    title: "Reload Settings",
                    category: "Settings",
                    aliases: &["config", "json5"],
                    availability: CommandAvailability::Active,
                },
                CommandDefinition {
                    id: CommandId::NewTab,
                    title: "New Tab",
                    category: "Terminal",
                    aliases: &["shell"],
                    availability: CommandAvailability::Active,
                },
                CommandDefinition {
                    id: CommandId::ClosePane,
                    title: "Close Pane",
                    category: "Terminal",
                    aliases: &["close tab"],
                    availability: CommandAvailability::Active,
                },
                CommandDefinition {
                    id: CommandId::SplitPane,
                    title: "Split Pane",
                    category: "Layout",
                    aliases: &["split right", "split down"],
                    availability: CommandAvailability::Active,
                },
                CommandDefinition {
                    id: CommandId::FindInPane,
                    title: "Find in Pane",
                    category: "Terminal",
                    aliases: &["search", "find"],
                    availability: CommandAvailability::Active,
                },
                CommandDefinition {
                    id: CommandId::OpenResourceDashboard,
                    title: "Open Resource Dashboard",
                    category: "Resources",
                    aliases: &["cpu", "rss", "vram"],
                    availability: CommandAvailability::Active,
                },
                // Phase 3 stubs
                CommandDefinition {
                    id: CommandId::CmdK,
                    title: "Cmd-K (Phase 3)",
                    category: "Developer UX",
                    aliases: &["ai", "ask", "explain"],
                    availability: CommandAvailability::DisabledUntilPhase3,
                },
                CommandDefinition {
                    id: CommandId::SmartHistory,
                    title: "Smart History (Phase 3)",
                    category: "History",
                    aliases: &["history", "previous commands"],
                    availability: CommandAvailability::DisabledUntilPhase3,
                },
                CommandDefinition {
                    id: CommandId::ExplainLastFailed,
                    title: "Explain Last Failed Command (Phase 3)",
                    category: "Developer UX",
                    aliases: &["error", "explain error"],
                    availability: CommandAvailability::DisabledUntilPhase3,
                },
                CommandDefinition {
                    id: CommandId::AttachContext,
                    title: "Attach Context (Phase 3)",
                    category: "Developer UX",
                    aliases: &["context", "attach"],
                    availability: CommandAvailability::DisabledUntilPhase3,
                },
                CommandDefinition {
                    id: CommandId::ToggleBackgroundJobs,
                    title: "Toggle Background Jobs (Phase 3)",
                    category: "Jobs",
                    aliases: &["jobs", "background", "tasks"],
                    availability: CommandAvailability::DisabledUntilPhase3,
                },
            ],
        }
    }
}

impl CommandPalette {
    #[must_use]
    pub fn all_commands(&self) -> &[CommandDefinition] {
        &self.commands
    }

    #[must_use]
    pub fn filter(&self, query: &str) -> Vec<&CommandDefinition> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return self.commands.iter().collect();
        }
        self.commands
            .iter()
            .filter(|cmd| cmd.matches(&query))
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Keyboard map — settings-backed
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
pub struct KeyboardMap;

impl KeyboardMap {
    // Kept as a `&self` method (not an associated fn) for call-site ergonomics
    // and API stability: callers invoke `keymap.shortcut_for(..)`. KeyboardMap is
    // a ZST, so the `&self` receiver is zero-cost.
    #[allow(clippy::unused_self, clippy::trivially_copy_pass_by_ref)]
    #[must_use]
    pub fn shortcut_for<'a>(
        &self,
        command: CommandId,
        keybindings: &'a KeybindingSettings,
    ) -> &'a str {
        match command {
            CommandId::OpenCommandPalette => &keybindings.command_palette,
            CommandId::ReloadSettings => "",
            CommandId::NewTab => &keybindings.new_tab,
            CommandId::ClosePane => &keybindings.close_pane,
            CommandId::SplitPane => &keybindings.split_pane,
            CommandId::FindInPane => &keybindings.find_in_pane,
            CommandId::OpenResourceDashboard => &keybindings.open_resource_dashboard,
            CommandId::CmdK => &keybindings.cmd_k,
            CommandId::SmartHistory => &keybindings.smart_history,
            CommandId::ExplainLastFailed => &keybindings.explain_last_failed,
            CommandId::AttachContext => &keybindings.attach_context,
            CommandId::ToggleBackgroundJobs => &keybindings.toggle_background_jobs,
        }
    }
}

// ---------------------------------------------------------------------------
// Palette state (query + selection)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
pub struct PaletteState {
    query: String,
    selected_index: usize,
}

impl PaletteState {
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    #[must_use]
    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.selected_index = 0;
    }

    pub fn select_next(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected_index = (self.selected_index + 1) % count;
    }

    pub fn select_prev(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        self.selected_index = if self.selected_index == 0 {
            count - 1
        } else {
            self.selected_index - 1
        };
    }

    pub fn reset(&mut self) {
        self.query.clear();
        self.selected_index = 0;
    }
}

// ---------------------------------------------------------------------------
// Onboarding
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingStep {
    Welcome,
    Shell,
    Appearance,
    ShellIntegration,
    AgentsDetected,
    PrivacyAndStorage,
    ResourceBudgets,
    Finish,
}

impl OnboardingStep {
    // Explicit per-step transitions kept for readability: the terminal
    // `Finish => Finish` self-loop is intentional and worth showing alongside
    // the `ResourceBudgets => Finish` edge even though both land on Finish.
    #[allow(clippy::match_same_arms)]
    fn next(self) -> Self {
        match self {
            Self::Welcome => Self::Shell,
            Self::Shell => Self::Appearance,
            Self::Appearance => Self::ShellIntegration,
            Self::ShellIntegration => Self::AgentsDetected,
            Self::AgentsDetected => Self::PrivacyAndStorage,
            Self::PrivacyAndStorage => Self::ResourceBudgets,
            Self::ResourceBudgets => Self::Finish,
            Self::Finish => Self::Finish,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedShell {
    pub name: String,
    pub available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedAgent {
    pub name: String,
    pub available: bool,
}

#[derive(Debug, Clone)]
pub struct OnboardingState {
    pub step: OnboardingStep,
    pub detected_shells: Vec<DetectedShell>,
    pub detected_agents: Vec<DetectedAgent>,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self::new()
    }
}

impl OnboardingState {
    #[must_use]
    pub fn new() -> Self {
        Self {
            step: OnboardingStep::Welcome,
            detected_shells: vec![],
            detected_agents: vec![],
        }
    }

    pub fn advance(&mut self) {
        self.step = self.step.next();
    }

    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.step == OnboardingStep::Finish
    }
}

// ---------------------------------------------------------------------------
// Shell messages
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum ShellMessage {
    NoOp,
    OpenCommandPalette,
    DismissCommandPalette,
    QueryChanged(String),
    PaletteSelectNext,
    PaletteSelectPrev,
    PaletteExecuteSelected,
    OnboardingAdvance,
    OnboardingFinish,
    AgentLifecycle {
        run_id: String,
        control: agent_sidebar::LifecycleControl,
    },
    AgentInterrupt {
        run_id: String,
    },
    ApprovalResolve {
        approval_id: u64,
        approve: bool,
    },
}

// ---------------------------------------------------------------------------
// Shell
// ---------------------------------------------------------------------------

// Each bool is an independent UI toggle (sidebar/dashboard/palette/onboarding)
// with its own message-driven lifecycle; bundling them into a flags type would
// obscure intent without changing behavior.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct BongTermShell {
    workspace_name: String,
    focus: ShellFocus,
    agent_sidebar_expanded: bool,
    resource_dashboard_expanded: bool,
    command_palette_open: bool,
    command_palette: CommandPalette,
    palette_state: PaletteState,
    keymap: KeyboardMap,
    settings: Settings,
    onboarding_active: bool,
    onboarding_state: OnboardingState,
}

impl Default for BongTermShell {
    fn default() -> Self {
        Self::with_settings(Settings::default())
    }
}

impl BongTermShell {
    #[must_use]
    pub fn with_settings(settings: Settings) -> Self {
        let onboarding_active = !settings.onboarding.completed;
        Self {
            workspace_name: "workspace".to_string(),
            focus: ShellFocus::Terminal,
            agent_sidebar_expanded: false,
            resource_dashboard_expanded: false,
            command_palette_open: false,
            command_palette: CommandPalette::default(),
            palette_state: PaletteState::default(),
            keymap: KeyboardMap,
            onboarding_active,
            onboarding_state: OnboardingState::new(),
            settings,
        }
    }

    #[must_use]
    pub fn is_onboarding_active(&self) -> bool {
        self.onboarding_active
    }

    #[must_use]
    pub fn onboarding_state(&self) -> &OnboardingState {
        &self.onboarding_state
    }

    #[must_use]
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    #[must_use]
    pub fn title(&self) -> String {
        format!("BongTerm - {}", self.workspace_name)
    }

    // Kept as a `&self` method for API stability (callers use
    // `shell.region_names()`); the region set is fixed and does not read state.
    #[allow(clippy::unused_self)]
    #[must_use]
    pub const fn region_names(&self) -> [&'static str; 7] {
        [
            "title-bar",
            "tab-strip",
            "agent-sidebar",
            "terminal-surface",
            "resource-dashboard",
            "status-bar",
            "command-palette",
        ]
    }

    #[must_use]
    pub const fn focus(&self) -> ShellFocus {
        self.focus
    }

    #[must_use]
    pub const fn agent_sidebar_expanded(&self) -> bool {
        self.agent_sidebar_expanded
    }

    #[must_use]
    pub const fn resource_dashboard_expanded(&self) -> bool {
        self.resource_dashboard_expanded
    }

    #[must_use]
    pub const fn command_palette_open(&self) -> bool {
        self.command_palette_open
    }

    #[must_use]
    pub fn palette_state(&self) -> &PaletteState {
        &self.palette_state
    }

    #[must_use]
    pub fn command_palette(&self) -> &CommandPalette {
        &self.command_palette
    }

    pub fn boot() -> (Self, Task<ShellMessage>) {
        (Self::default(), Task::none())
    }

    // match_same_arms: the empty `NoOp` arm is kept distinct from the empty
    //   agent/approval arms — they are semantically different messages.
    // collapsible_if: the nested `if let Some(cmd)` / `if active` is left
    //   un-collapsed so the `// Disabled commands` else-path comment stays put.
    #[allow(clippy::match_same_arms, clippy::collapsible_if)]
    pub fn update(&mut self, message: ShellMessage) -> Task<ShellMessage> {
        match message {
            ShellMessage::NoOp => {}
            ShellMessage::OpenCommandPalette => {
                self.command_palette_open = true;
                self.focus = ShellFocus::CommandPalette;
                self.palette_state.reset();
            }
            ShellMessage::DismissCommandPalette => {
                if self.command_palette_open {
                    self.command_palette_open = false;
                    self.focus = ShellFocus::Terminal;
                }
            }
            ShellMessage::QueryChanged(query) => {
                self.palette_state.set_query(query);
            }
            ShellMessage::PaletteSelectNext => {
                if self.command_palette_open {
                    let count = self
                        .command_palette
                        .filter(self.palette_state.query())
                        .len();
                    self.palette_state.select_next(count);
                }
            }
            ShellMessage::PaletteSelectPrev => {
                if self.command_palette_open {
                    let count = self
                        .command_palette
                        .filter(self.palette_state.query())
                        .len();
                    self.palette_state.select_prev(count);
                }
            }
            ShellMessage::PaletteExecuteSelected => {
                if self.command_palette_open {
                    let results = self.command_palette.filter(self.palette_state.query());
                    if let Some(cmd) = results.get(self.palette_state.selected_index()) {
                        if cmd.availability == CommandAvailability::Active {
                            self.command_palette_open = false;
                            self.focus = ShellFocus::Terminal;
                            self.palette_state.reset();
                            // TODO(1.B+): route by command id once subsystems land
                        }
                        // Disabled commands: keep palette open, no action
                    }
                }
            }
            ShellMessage::OnboardingAdvance => {
                self.onboarding_state.advance();
            }
            ShellMessage::OnboardingFinish => {
                self.settings.onboarding.completed = true;
                self.onboarding_active = false;
            }
            ShellMessage::AgentLifecycle { .. }
            | ShellMessage::AgentInterrupt { .. }
            | ShellMessage::ApprovalResolve { .. } => {
                // Routed by app layer to agent supervisor; shell view is no-op.
            }
        }

        Task::none()
    }

    // `&self` is required: this is passed as `fn(&State) -> Subscription` to
    // `iced::application(..).subscription(BongTermShell::subscription)`. The
    // event subscription is global, so the receiver is unused but mandatory.
    #[allow(clippy::unused_self)]
    pub fn subscription(&self) -> iced::Subscription<ShellMessage> {
        use iced::event::Event;
        use iced::keyboard::{self, Key, key::Named};

        // Spike S3b confirmed: listen_raw(event, status, window) delivers all events.
        iced::event::listen_raw(|event, _status, _window| {
            if let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
                match key.as_ref() {
                    Key::Named(Named::Escape) => {
                        return Some(ShellMessage::DismissCommandPalette);
                    }
                    Key::Named(Named::ArrowUp) => {
                        return Some(ShellMessage::PaletteSelectPrev);
                    }
                    Key::Named(Named::ArrowDown) => {
                        return Some(ShellMessage::PaletteSelectNext);
                    }
                    Key::Named(Named::Enter) => {
                        return Some(ShellMessage::PaletteExecuteSelected);
                    }
                    Key::Character("p")
                        if modifiers == keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT =>
                    {
                        return Some(ShellMessage::OpenCommandPalette);
                    }
                    _ => {}
                }
            }
            None
        })
    }

    fn main_view(&self) -> Element<'_, ShellMessage> {
        let title_bar = text(self.title()).size(16);
        let tab_strip = row![text("[PowerShell - workspace]"), text("[+]")].spacing(8);
        let body = row![
            shell_panel("Agents", "collapsed"),
            container(text("Terminal surface\n\nshell prompt appears here"))
                .width(Length::Fill)
                .height(Length::Fill),
            shell_panel("Resources", "collapsed")
        ]
        .spacing(8)
        .height(Length::Fill);
        let status_bar = text("shell ready | workspace | resources ok").size(12);

        let base: Element<'_, ShellMessage> = column![title_bar, tab_strip, body, status_bar]
            .spacing(8)
            .padding(12)
            .into();

        if self.command_palette_open {
            stack![base, self.palette_overlay()].into()
        } else {
            base
        }
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, ShellMessage> {
        let main = self.main_view();
        if self.onboarding_active {
            stack![main, self.onboarding_overlay()].into()
        } else {
            main
        }
    }

    fn palette_overlay(&self) -> Element<'_, ShellMessage> {
        let input = text_input("Search commands…", self.palette_state.query())
            .on_input(ShellMessage::QueryChanged)
            .on_submit(ShellMessage::PaletteExecuteSelected)
            .padding(8);

        let keybindings = &self.settings.keybindings;
        let results = self.command_palette.filter(self.palette_state.query());
        let selected = self.palette_state.selected_index();

        let items: Vec<Element<'_, ShellMessage>> = results
            .iter()
            .enumerate()
            .map(|(i, cmd)| palette_row(cmd, i == selected, keybindings, &self.keymap))
            .collect();

        let result_list = column(items).spacing(2);

        let palette_box = container(column![input, result_list].spacing(4))
            .width(540)
            .padding(8)
            .style(|_theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.15, 0.15, 0.18,
                ))),
                border: iced::Border {
                    color: iced::Color::from_rgb(0.35, 0.35, 0.45),
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            });

        container(palette_box)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Top)
            .padding(iced::Padding {
                top: 64.0,
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            })
            .into()
    }

    fn onboarding_overlay(&self) -> Element<'_, ShellMessage> {
        let step_label = match self.onboarding_state.step {
            OnboardingStep::Welcome => "Welcome",
            OnboardingStep::Shell => "Shell",
            OnboardingStep::Appearance => "Appearance",
            OnboardingStep::ShellIntegration => "Shell Integration",
            OnboardingStep::AgentsDetected => "Agents Detected",
            OnboardingStep::PrivacyAndStorage => "Privacy & Storage",
            OnboardingStep::ResourceBudgets => "Resource Budgets",
            OnboardingStep::Finish => "Finish",
        };

        let is_finish = self.onboarding_state.is_finished();
        let next_btn = if is_finish {
            button(text("Get Started")).on_press(ShellMessage::OnboardingFinish)
        } else {
            button(text("Next")).on_press(ShellMessage::OnboardingAdvance)
        };

        let body = self.onboarding_step_body();

        let card = container(column![text(step_label).size(20), body, next_btn,].spacing(16))
            .width(480)
            .padding(24)
            .style(|_theme: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(iced::Color::from_rgb(
                    0.12, 0.12, 0.15,
                ))),
                border: iced::Border {
                    color: iced::Color::from_rgb(0.35, 0.35, 0.45),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            });

        container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(iced::alignment::Horizontal::Center)
            .align_y(iced::alignment::Vertical::Center)
            .into()
    }

    fn onboarding_step_body(&self) -> Element<'_, ShellMessage> {
        match self.onboarding_state.step {
            OnboardingStep::Welcome => {
                text("BongTerm — run CLI agents in parallel worktrees.").into()
            }
            OnboardingStep::Shell => {
                let shells = &self.onboarding_state.detected_shells;
                if shells.is_empty() {
                    text("No shells detected. You can configure shells later in settings.").into()
                } else {
                    let items: Vec<Element<'_, ShellMessage>> = shells
                        .iter()
                        .map(|s| {
                            let label = if s.available {
                                format!("{} (available)", s.name)
                            } else {
                                format!("{} (unavailable)", s.name)
                            };
                            text(label).into()
                        })
                        .collect();
                    column(items).spacing(4).into()
                }
            }
            OnboardingStep::Appearance => {
                text("Theme and font settings can be adjusted in settings.json5.").into()
            }
            OnboardingStep::ShellIntegration => {
                text("Shell integration enables command blocks and exit-code tracking.").into()
            }
            OnboardingStep::AgentsDetected => {
                let agents = &self.onboarding_state.detected_agents;
                if agents.is_empty() {
                    text("No agents detected. Agents can be configured later.").into()
                } else {
                    let items: Vec<Element<'_, ShellMessage>> = agents
                        .iter()
                        .map(|a| {
                            let label = if a.available {
                                format!("{} (available)", a.name)
                            } else {
                                format!("{} (unavailable)", a.name)
                            };
                            text(label).into()
                        })
                        .collect();
                    column(items).spacing(4).into()
                }
            }
            OnboardingStep::PrivacyAndStorage => {
                text("Telemetry is off by default. Local storage only.").into()
            }
            OnboardingStep::ResourceBudgets => {
                text("Default resource budgets apply. Adjust in settings.").into()
            }
            OnboardingStep::Finish => text("Setup complete. Open a shell to get started.").into(),
        }
    }

    // `&self` is required: passed as `fn(&State) -> Theme` to
    // `iced::application(..).theme(BongTermShell::theme)`. The theme is fixed and
    // does not read state, but the receiver is mandatory for the wiring.
    #[allow(clippy::unused_self)]
    #[must_use]
    pub const fn theme(&self) -> Theme {
        Theme::Dark
    }
}

fn palette_row<'a>(
    cmd: &'a CommandDefinition,
    selected: bool,
    keybindings: &'a KeybindingSettings,
    keymap: &'a KeyboardMap,
) -> Element<'a, ShellMessage> {
    let shortcut = keymap.shortcut_for(cmd.id, keybindings);
    let disabled = cmd.availability == CommandAvailability::DisabledUntilPhase3;

    let title_str = cmd.title;
    let label = text(title_str).size(14);
    let category = text(cmd.category).size(12);
    let shortcut_label = text(shortcut).size(12);

    let row_content = row![category, label.width(Length::Fill), shortcut_label]
        .spacing(8)
        .padding(6);

    container(row_content)
        .width(Length::Fill)
        .style(move |_theme: &Theme| {
            let base_bg = if selected {
                Some(iced::Background::Color(iced::Color::from_rgb(
                    0.2, 0.3, 0.5,
                )))
            } else {
                None
            };
            let text_color = if disabled {
                Some(iced::Color::from_rgb(0.5, 0.5, 0.5))
            } else {
                None
            };
            iced::widget::container::Style {
                background: base_bg,
                text_color,
                ..Default::default()
            }
        })
        .into()
}

/// Run the `BongTerm` shell application to completion.
///
/// # Errors
///
/// Returns an [`iced::Error`] if the windowing/graphics backend fails to
/// initialize or the application event loop terminates abnormally.
pub fn run_shell() -> ShellResult {
    iced::application(
        BongTermShell::boot,
        BongTermShell::update,
        BongTermShell::view,
    )
    .title(BongTermShell::title)
    .theme(BongTermShell::theme)
    .subscription(BongTermShell::subscription)
    .run()
}

fn shell_panel<'a>(title: &'a str, state: &'a str) -> Element<'a, ShellMessage> {
    container(column![text(title).size(14), text(state).size(12)])
        .width(160)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
#[allow(unused_must_use)]
mod tests {
    use super::*;
    use bongterm_settings::KeybindingSettings;

    #[test]
    fn crate_compiles() {}

    #[test]
    fn shell_default_title_names_product_and_workspace() {
        let shell = BongTermShell::default();
        assert_eq!(shell.title(), "BongTerm - workspace");
    }

    #[test]
    fn shell_contract_has_phase1_regions() {
        let shell = BongTermShell::default();
        assert_eq!(
            shell.region_names(),
            [
                "title-bar",
                "tab-strip",
                "agent-sidebar",
                "terminal-surface",
                "resource-dashboard",
                "status-bar",
                "command-palette"
            ]
        );
    }

    #[test]
    fn shell_starts_with_terminal_focused_and_panels_collapsed() {
        let shell = BongTermShell::default();
        assert_eq!(shell.focus(), ShellFocus::Terminal);
        assert!(!shell.agent_sidebar_expanded());
        assert!(!shell.resource_dashboard_expanded());
    }

    #[test]
    fn default_keymap_reads_phase1_bindings_from_settings() {
        let settings = KeybindingSettings::default();
        let keymap = KeyboardMap;
        assert_eq!(
            keymap.shortcut_for(CommandId::OpenCommandPalette, &settings),
            "Ctrl+Shift+P"
        );
        assert_eq!(
            keymap.shortcut_for(CommandId::NewTab, &settings),
            "Ctrl+Shift+T"
        );
        assert_eq!(
            keymap.shortcut_for(CommandId::ClosePane, &settings),
            "Ctrl+Shift+W"
        );
        assert_eq!(
            keymap.shortcut_for(CommandId::SplitPane, &settings),
            "Alt+Shift+D"
        );
        assert_eq!(
            keymap.shortcut_for(CommandId::FindInPane, &settings),
            "Ctrl+F"
        );
        assert_eq!(
            keymap.shortcut_for(CommandId::OpenResourceDashboard, &settings),
            "Ctrl+Shift+R"
        );
        assert_eq!(keymap.shortcut_for(CommandId::CmdK, &settings), "Ctrl+K");
        assert_eq!(
            keymap.shortcut_for(CommandId::SmartHistory, &settings),
            "Ctrl+R"
        );
    }

    #[test]
    fn command_palette_filters_by_title_category_and_alias() {
        let palette = CommandPalette::default();
        let reload_ids: Vec<CommandId> = palette.filter("reload").iter().map(|c| c.id).collect();
        assert!(reload_ids.contains(&CommandId::ReloadSettings));
        let layout_ids: Vec<CommandId> = palette.filter("layout").iter().map(|c| c.id).collect();
        assert!(layout_ids.contains(&CommandId::SplitPane));
        let resource_ids: Vec<CommandId> =
            palette.filter("resources").iter().map(|c| c.id).collect();
        assert!(resource_ids.contains(&CommandId::OpenResourceDashboard));
    }

    #[test]
    fn shell_message_toggles_palette_focus() {
        let mut shell = BongTermShell::default();
        shell.update(ShellMessage::OpenCommandPalette);
        assert!(shell.command_palette_open());
        assert_eq!(shell.focus(), ShellFocus::CommandPalette);
        shell.update(ShellMessage::DismissCommandPalette);
        assert!(!shell.command_palette_open());
        assert_eq!(shell.focus(), ShellFocus::Terminal);
    }

    // --- 1.A.3 new tests ---

    #[test]
    fn palette_state_starts_empty_with_zero_selection() {
        let state = PaletteState::default();
        assert_eq!(state.query(), "");
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn palette_state_set_query_resets_selection() {
        let mut state = PaletteState::default();
        state.select_next(5);
        state.set_query("rel".to_string());
        assert_eq!(state.query(), "rel");
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn palette_state_select_next_advances_and_wraps() {
        let mut state = PaletteState::default();
        state.select_next(3);
        assert_eq!(state.selected_index(), 1);
        state.select_next(3);
        assert_eq!(state.selected_index(), 2);
        state.select_next(3);
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn palette_state_select_prev_decrements_and_wraps() {
        let mut state = PaletteState::default();
        state.select_prev(3);
        assert_eq!(state.selected_index(), 2);
        state.select_prev(3);
        assert_eq!(state.selected_index(), 1);
    }

    #[test]
    fn palette_state_reset_clears_query_and_selection() {
        let mut state = PaletteState::default();
        state.set_query("test".to_string());
        state.select_next(5);
        state.reset();
        assert_eq!(state.query(), "");
        assert_eq!(state.selected_index(), 0);
    }

    #[test]
    fn command_palette_contains_all_phase1_and_phase3_stub_commands() {
        let palette = CommandPalette::default();
        let ids: Vec<CommandId> = palette.all_commands().iter().map(|c| c.id).collect();
        assert!(ids.contains(&CommandId::OpenCommandPalette));
        assert!(ids.contains(&CommandId::ReloadSettings));
        assert!(ids.contains(&CommandId::NewTab));
        assert!(ids.contains(&CommandId::ClosePane));
        assert!(ids.contains(&CommandId::SplitPane));
        assert!(ids.contains(&CommandId::FindInPane));
        assert!(ids.contains(&CommandId::OpenResourceDashboard));
        assert!(ids.contains(&CommandId::CmdK));
        assert!(ids.contains(&CommandId::SmartHistory));
        assert!(ids.contains(&CommandId::ExplainLastFailed));
        assert!(ids.contains(&CommandId::AttachContext));
        assert!(ids.contains(&CommandId::ToggleBackgroundJobs));
    }

    #[test]
    fn phase3_stub_commands_are_marked_disabled() {
        let palette = CommandPalette::default();
        for id in [
            CommandId::CmdK,
            CommandId::SmartHistory,
            CommandId::ExplainLastFailed,
            CommandId::AttachContext,
            CommandId::ToggleBackgroundJobs,
        ] {
            let cmd = palette.all_commands().iter().find(|c| c.id == id).unwrap();
            assert_eq!(
                cmd.availability,
                CommandAvailability::DisabledUntilPhase3,
                "{id:?} should be DisabledUntilPhase3"
            );
        }
    }

    #[test]
    fn phase1_commands_are_active() {
        let palette = CommandPalette::default();
        for id in [
            CommandId::ReloadSettings,
            CommandId::NewTab,
            CommandId::ClosePane,
            CommandId::SplitPane,
            CommandId::FindInPane,
            CommandId::OpenResourceDashboard,
        ] {
            let cmd = palette.all_commands().iter().find(|c| c.id == id).unwrap();
            assert_eq!(
                cmd.availability,
                CommandAvailability::Active,
                "{id:?} should be Active"
            );
        }
    }

    #[test]
    fn shell_open_palette_resets_palette_state() {
        let mut shell = BongTermShell::default();
        shell.update(ShellMessage::QueryChanged("test".to_string()));
        shell.update(ShellMessage::OpenCommandPalette);
        assert_eq!(shell.palette_state().query(), "");
        assert_eq!(shell.palette_state().selected_index(), 0);
    }

    #[test]
    fn shell_query_changed_updates_palette_query() {
        let mut shell = BongTermShell::default();
        shell.update(ShellMessage::OpenCommandPalette);
        shell.update(ShellMessage::QueryChanged("split".to_string()));
        assert_eq!(shell.palette_state().query(), "split");
    }

    #[test]
    fn shell_palette_select_next_advances_selection() {
        let mut shell = BongTermShell::default();
        shell.update(ShellMessage::OpenCommandPalette);
        shell.update(ShellMessage::PaletteSelectNext);
        assert_eq!(shell.palette_state().selected_index(), 1);
    }

    #[test]
    fn shell_palette_select_prev_wraps_selection_to_last() {
        let mut shell = BongTermShell::default();
        shell.update(ShellMessage::OpenCommandPalette);
        let count = shell.command_palette().filter("").len();
        shell.update(ShellMessage::PaletteSelectPrev);
        assert_eq!(shell.palette_state().selected_index(), count - 1);
    }

    #[test]
    fn shell_execute_active_command_closes_palette() {
        let mut shell = BongTermShell::default();
        shell.update(ShellMessage::OpenCommandPalette);
        shell.update(ShellMessage::PaletteExecuteSelected);
        assert!(!shell.command_palette_open());
        assert_eq!(shell.focus(), ShellFocus::Terminal);
    }

    #[test]
    fn shell_execute_disabled_command_keeps_palette_open() {
        let mut shell = BongTermShell::default();
        shell.update(ShellMessage::OpenCommandPalette);
        let palette = CommandPalette::default();
        let all = palette.all_commands();
        let disabled_pos = all
            .iter()
            .position(|c| c.availability == CommandAvailability::DisabledUntilPhase3)
            .unwrap();
        for _ in 0..disabled_pos {
            shell.update(ShellMessage::PaletteSelectNext);
        }
        shell.update(ShellMessage::PaletteExecuteSelected);
        assert!(
            shell.command_palette_open(),
            "palette must stay open for disabled command"
        );
    }

    // --- 1.A.4 onboarding tests ---

    #[test]
    fn onboarding_state_starts_at_welcome_step() {
        let state = OnboardingState::new();
        assert_eq!(state.step, OnboardingStep::Welcome);
        assert!(!state.is_finished());
    }

    #[test]
    fn onboarding_advances_through_all_steps_in_order() {
        let mut state = OnboardingState::new();
        let expected = [
            OnboardingStep::Welcome,
            OnboardingStep::Shell,
            OnboardingStep::Appearance,
            OnboardingStep::ShellIntegration,
            OnboardingStep::AgentsDetected,
            OnboardingStep::PrivacyAndStorage,
            OnboardingStep::ResourceBudgets,
            OnboardingStep::Finish,
        ];
        for &step in &expected {
            assert_eq!(state.step, step);
            if step != OnboardingStep::Finish {
                state.advance();
            }
        }
        assert!(state.is_finished());
    }

    #[test]
    fn onboarding_completes_without_optional_integrations() {
        // Acceptance criterion 1: user can finish without optional integrations.
        let mut state = OnboardingState::new();
        // Advance through all 7 transitions (Welcome->...->Finish)
        for _ in 0..7 {
            state.advance();
        }
        assert!(state.is_finished());
    }

    #[test]
    fn onboarding_missing_shells_and_agents_advance_without_error() {
        // Acceptance criterion 2: missing shell or agent → disabled state, not error.
        // Empty detected lists; advancing through Shell and AgentsDetected must not panic.
        let mut state = OnboardingState::new();
        assert!(state.detected_shells.is_empty());
        assert!(state.detected_agents.is_empty());
        // Advance through all steps
        for _ in 0..7 {
            state.advance();
        }
        assert!(state.is_finished());
    }

    #[test]
    fn shell_active_when_onboarding_not_completed() {
        use bongterm_settings::OnboardingSettings;
        let settings = Settings {
            onboarding: OnboardingSettings {
                completed: false,
                ..OnboardingSettings::default()
            },
            ..Settings::default()
        };
        let shell = BongTermShell::with_settings(settings);
        assert!(shell.is_onboarding_active());
    }

    #[test]
    fn shell_inactive_when_onboarding_completed() {
        use bongterm_settings::OnboardingSettings;
        let settings = Settings {
            onboarding: OnboardingSettings {
                completed: true,
                ..OnboardingSettings::default()
            },
            ..Settings::default()
        };
        let shell = BongTermShell::with_settings(settings);
        assert!(!shell.is_onboarding_active());
    }

    #[test]
    fn shell_onboarding_advance_moves_to_next_step() {
        let mut shell = BongTermShell::default();
        assert_eq!(shell.onboarding_state().step, OnboardingStep::Welcome);
        shell.update(ShellMessage::OnboardingAdvance);
        assert_eq!(shell.onboarding_state().step, OnboardingStep::Shell);
    }

    #[test]
    fn shell_onboarding_finish_deactivates_and_marks_completed() {
        let mut shell = BongTermShell::default();
        assert!(shell.is_onboarding_active());
        shell.update(ShellMessage::OnboardingFinish);
        assert!(!shell.is_onboarding_active());
        assert!(shell.settings().onboarding.completed);
    }
}
