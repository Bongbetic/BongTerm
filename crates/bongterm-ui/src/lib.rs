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

/// Shell outer padding in logical pixels.
pub const SHELL_OUTER_PADDING: u16 = 12;
/// Gap between the agent panel, terminal surface, and resource panel.
pub const SHELL_BODY_SPACING: u16 = 8;
/// Vertical gap between title/tab/body/status regions.
pub const SHELL_COLUMN_SPACING: u16 = 8;
/// Side-panel width used by both panel views.
pub const SHELL_SIDE_PANEL_WIDTH: f32 = 220.0;
/// Side-panel internal padding in logical pixels.
pub const SHELL_PANEL_PADDING: u16 = 8;
/// Fixed shell chrome heights used by the layout helper and the view.
pub const SHELL_TITLE_BAR_HEIGHT: f32 = 24.0;
pub const SHELL_TAB_STRIP_HEIGHT: f32 = 24.0;
pub const SHELL_STATUS_BAR_HEIGHT: f32 = 20.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TerminalSurfaceSize {
    pub width: f32,
    pub height: f32,
}

// ---------------------------------------------------------------------------
// Resource dashboard view-model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDashboardVm {
    pub total_rss: String,
    pub total_cpu_pct: String,
    pub vram: Option<String>,
    pub rows: Vec<ResourceRowVm>,
    pub is_stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRowVm {
    pub category: String,
    pub pid: u32,
    pub rss: String,
    pub cpu_pct: String,
}

impl ResourceRowVm {
    #[must_use]
    pub fn title_line(&self) -> &str {
        &self.category
    }

    #[must_use]
    pub fn metrics_line(&self) -> String {
        format!("pid {} | RSS {} | CPU {}", self.pid, self.rss, self.cpu_pct)
    }
}

impl Default for ResourceDashboardVm {
    fn default() -> Self {
        Self {
            total_rss: "0 B".to_string(),
            total_cpu_pct: "0.0%".to_string(),
            vram: None,
            rows: vec![],
            is_stale: false,
        }
    }
}

impl ResourceDashboardVm {
    #[must_use]
    pub fn view(&self) -> Element<'_, ShellMessage> {
        let freshness = if self.is_stale { "stale" } else { "live" };
        let vram = self.vram.as_deref().unwrap_or("Metric unavailable from OS");

        let mut col = column![
            text("Resources").size(16),
            text(format!(
                "RSS {} | CPU {} | {}",
                self.total_rss, self.total_cpu_pct, freshness
            ))
            .size(12),
            text(format!("VRAM {vram}")).size(12),
        ]
        .spacing(8);

        if self.rows.is_empty() {
            col = col.push(text("No process samples").size(12));
        } else {
            for row_vm in self.rows.iter().take(6) {
                col = col.push(
                    column![
                        text(row_vm.title_line()).size(13),
                        text(row_vm.metrics_line()).size(12)
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                );
            }
        }

        container(col)
            .width(Length::Fixed(SHELL_SIDE_PANEL_WIDTH))
            .height(Length::Fill)
            .padding(SHELL_PANEL_PADDING)
            .into()
    }
}

// ---------------------------------------------------------------------------
// Focus
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellFocus {
    Terminal,
    CommandPalette,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MvpPanel {
    CmdK,
    SmartHistory,
    Explainer,
    Snippets,
    BackgroundJobs,
    CommandBlocks,
    Mcp,
    Diagnostics,
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
    CmdK,
    SmartHistory,
    ExplainLastFailed,
    AttachContext,
    ToggleBackgroundJobs,
    OpenSnippets,
    OpenCommandBlocks,
    OpenMcpPanel,
    OpenDiagnostics,
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
        let mut commands = phase1_commands();
        commands.extend(mvp_commands());
        Self { commands }
    }
}

fn active_command(
    id: CommandId,
    title: &'static str,
    category: &'static str,
    aliases: &'static [&'static str],
) -> CommandDefinition {
    CommandDefinition {
        id,
        title,
        category,
        aliases,
        availability: CommandAvailability::Active,
    }
}

fn phase1_commands() -> Vec<CommandDefinition> {
    vec![
        active_command(
            CommandId::OpenCommandPalette,
            "Open Command Palette",
            "Shell",
            &["palette", "commands"],
        ),
        active_command(
            CommandId::ReloadSettings,
            "Reload Settings",
            "Settings",
            &["config", "json5"],
        ),
        active_command(CommandId::NewTab, "New Tab", "Terminal", &["shell"]),
        active_command(
            CommandId::ClosePane,
            "Close Pane",
            "Terminal",
            &["close tab"],
        ),
        active_command(
            CommandId::SplitPane,
            "Split Pane",
            "Layout",
            &["split right", "split down"],
        ),
        active_command(
            CommandId::FindInPane,
            "Find in Pane",
            "Terminal",
            &["search", "find"],
        ),
        active_command(
            CommandId::OpenResourceDashboard,
            "Open Resource Dashboard",
            "Resources",
            &["cpu", "rss", "vram"],
        ),
    ]
}

fn mvp_commands() -> Vec<CommandDefinition> {
    vec![
        active_command(
            CommandId::CmdK,
            "Cmd-K",
            "Developer UX",
            &["ai", "ask", "explain"],
        ),
        active_command(
            CommandId::SmartHistory,
            "Smart History",
            "History",
            &["history", "previous commands", "filters"],
        ),
        active_command(
            CommandId::ExplainLastFailed,
            "Explain Last Failed Command",
            "Developer UX",
            &["error", "explain error"],
        ),
        active_command(
            CommandId::AttachContext,
            "Attach Context",
            "Developer UX",
            &["context", "attach", "command block"],
        ),
        active_command(
            CommandId::ToggleBackgroundJobs,
            "Background Jobs",
            "Jobs",
            &["jobs", "background", "tasks"],
        ),
        active_command(
            CommandId::OpenSnippets,
            "Snippets",
            "Developer UX",
            &["snippet", "parameter prompt", "templates"],
        ),
        active_command(
            CommandId::OpenCommandBlocks,
            "Command Blocks",
            "Terminal",
            &["blocks", "confidence", "rerun", "export"],
        ),
        active_command(
            CommandId::OpenMcpPanel,
            "MCP Servers",
            "MCP",
            &["mcp", "json import", "permissions", "logs"],
        ),
        active_command(
            CommandId::OpenDiagnostics,
            "Diagnostics Export",
            "Diagnostics",
            &["telemetry", "logs", "redaction", "export"],
        ),
    ]
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
            CommandId::ReloadSettings
            | CommandId::OpenSnippets
            | CommandId::OpenCommandBlocks
            | CommandId::OpenMcpPanel
            | CommandId::OpenDiagnostics => "",
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
    OpenPanel(MvpPanel),
    ClosePanel,
    CmdKPromptChanged(String),
    CmdKRequestPreview,
    CmdKConfirmRun,
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
    agent_sidebar: agent_sidebar::AgentSidebarVm,
    resource_dashboard: ResourceDashboardVm,
    command_palette_open: bool,
    command_palette: CommandPalette,
    palette_state: PaletteState,
    active_panel: Option<MvpPanel>,
    cmdk_prompt: String,
    cmdk_preview: Option<String>,
    pending_terminal_command: Option<String>,
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
            agent_sidebar: agent_sidebar::AgentSidebarVm {
                agents: vec![],
                approvals: vec![],
            },
            resource_dashboard: ResourceDashboardVm::default(),
            command_palette_open: false,
            command_palette: CommandPalette::default(),
            palette_state: PaletteState::default(),
            active_panel: None,
            cmdk_prompt: String::new(),
            cmdk_preview: None,
            pending_terminal_command: None,
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

    #[must_use]
    pub const fn active_panel(&self) -> Option<MvpPanel> {
        self.active_panel
    }

    #[must_use]
    pub fn cmdk_preview(&self) -> Option<&str> {
        self.cmdk_preview.as_deref()
    }

    pub fn take_pending_terminal_command(&mut self) -> Option<String> {
        self.pending_terminal_command.take()
    }

    pub fn set_panel_data(
        &mut self,
        agent_sidebar: agent_sidebar::AgentSidebarVm,
        resource_dashboard: ResourceDashboardVm,
    ) {
        self.agent_sidebar = agent_sidebar;
        self.resource_dashboard = resource_dashboard;
    }

    #[must_use]
    pub fn agent_sidebar_snapshot(&self) -> &agent_sidebar::AgentSidebarVm {
        &self.agent_sidebar
    }

    #[must_use]
    pub const fn resource_dashboard_snapshot(&self) -> &ResourceDashboardVm {
        &self.resource_dashboard
    }

    #[must_use]
    pub fn terminal_surface_size_for_window(width: f32, height: f32) -> TerminalSurfaceSize {
        let horizontal_chrome = (2.0 * f32::from(SHELL_OUTER_PADDING))
            + (2.0 * SHELL_SIDE_PANEL_WIDTH)
            + (2.0 * f32::from(SHELL_BODY_SPACING));
        let vertical_chrome = (2.0 * f32::from(SHELL_OUTER_PADDING))
            + SHELL_TITLE_BAR_HEIGHT
            + SHELL_TAB_STRIP_HEIGHT
            + SHELL_STATUS_BAR_HEIGHT
            + (3.0 * f32::from(SHELL_COLUMN_SPACING));

        TerminalSurfaceSize {
            width: (width - horizontal_chrome).max(1.0),
            height: (height - vertical_chrome).max(1.0),
        }
    }

    pub fn boot() -> (Self, Task<ShellMessage>) {
        (Self::default(), Task::none())
    }

    // match_same_arms: the empty `NoOp` arm is kept distinct from the empty
    //   agent/approval arms — they are semantically different messages.
    #[allow(clippy::match_same_arms)]
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
                    if let Some(cmd) = results.get(self.palette_state.selected_index())
                        && cmd.availability == CommandAvailability::Active
                    {
                        if let Some(panel) = panel_for_command(cmd.id) {
                            self.open_panel(panel);
                        } else if cmd.id == CommandId::OpenResourceDashboard {
                            self.resource_dashboard_expanded = !self.resource_dashboard_expanded;
                        }
                        self.command_palette_open = false;
                        self.focus = ShellFocus::Terminal;
                        self.palette_state.reset();
                    }
                }
            }
            ShellMessage::OpenPanel(panel) => {
                self.open_panel(panel);
            }
            ShellMessage::ClosePanel => {
                self.active_panel = None;
                self.focus = ShellFocus::Terminal;
            }
            ShellMessage::CmdKPromptChanged(prompt) => {
                self.active_panel = Some(MvpPanel::CmdK);
                self.cmdk_prompt = prompt;
                self.cmdk_preview = None;
            }
            ShellMessage::CmdKRequestPreview => {
                self.active_panel = Some(MvpPanel::CmdK);
                self.cmdk_preview = cmdk_preview_for_prompt(&self.cmdk_prompt);
            }
            ShellMessage::CmdKConfirmRun => {
                if let Some(command) = self.cmdk_preview.clone() {
                    self.pending_terminal_command = Some(command);
                    self.active_panel = None;
                    self.focus = ShellFocus::Terminal;
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

    fn open_panel(&mut self, panel: MvpPanel) {
        self.active_panel = Some(panel);
        self.focus = ShellFocus::Terminal;
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
        let terminal = container(text("Terminal surface\n\nshell prompt appears here"))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        self.view_with_terminal(terminal, std::convert::identity)
    }

    pub fn view_with_terminal<'a, Message, F>(
        &'a self,
        terminal: Element<'a, Message>,
        map_shell: F,
    ) -> Element<'a, Message>
    where
        Message: Clone + 'a,
        F: Fn(ShellMessage) -> Message + Copy + 'a,
    {
        let title_bar: Element<'a, Message> = container(text(self.title()).size(16))
            .height(Length::Fixed(SHELL_TITLE_BAR_HEIGHT))
            .into();
        let tab_strip: Element<'a, Message> = container(
            row![text("[PowerShell - workspace]"), text("[+]")]
                .spacing(u32::from(SHELL_BODY_SPACING)),
        )
        .height(Length::Fixed(SHELL_TAB_STRIP_HEIGHT))
        .into();
        let body: Element<'a, Message> = row![
            self.agent_sidebar.view().map(map_shell),
            container(terminal).width(Length::Fill).height(Length::Fill),
            self.resource_dashboard.view().map(map_shell)
        ]
        .spacing(u32::from(SHELL_BODY_SPACING))
        .height(Length::Fill)
        .into();
        let status_bar: Element<'a, Message> =
            container(text("shell ready | workspace | resources ok").size(12))
                .height(Length::Fixed(SHELL_STATUS_BAR_HEIGHT))
                .into();

        let base: Element<'a, Message> = column![title_bar, tab_strip, body, status_bar]
            .spacing(u32::from(SHELL_COLUMN_SPACING))
            .padding(SHELL_OUTER_PADDING)
            .into();

        let mut layered = stack![base];
        if self.active_panel.is_some() {
            layered = layered.push(self.mvp_panel_overlay().map(map_shell));
        }
        if self.command_palette_open {
            layered = layered.push(self.palette_overlay().map(map_shell));
        }
        if self.onboarding_active {
            layered = layered.push(self.onboarding_overlay().map(map_shell));
        }
        layered.into()
    }

    #[must_use]
    pub fn view(&self) -> Element<'_, ShellMessage> {
        self.main_view()
    }

    fn mvp_panel_overlay(&self) -> Element<'_, ShellMessage> {
        let Some(panel) = self.active_panel else {
            return container(text("")).into();
        };

        let title = match panel {
            MvpPanel::CmdK => "Cmd-K",
            MvpPanel::SmartHistory => "Smart History",
            MvpPanel::Explainer => "Explain Last Failed",
            MvpPanel::Snippets => "Snippets",
            MvpPanel::BackgroundJobs => "Background Jobs",
            MvpPanel::CommandBlocks => "Command Blocks",
            MvpPanel::Mcp => "MCP Servers",
            MvpPanel::Diagnostics => "Diagnostics Export",
        };

        let body = match panel {
            MvpPanel::CmdK => self.cmdk_panel_body(),
            MvpPanel::SmartHistory => panel_lines(&[
                "Filters: cwd: branch: agent: exit: time: shell: duration:",
                "Selection inserts the chosen command into the active terminal.",
                "No history command auto-runs from this surface.",
            ]),
            MvpPanel::Explainer => panel_lines(&[
                "Explainer is scoped to the last failed command block.",
                "Unavailable when no failed block is selected.",
                "Output is previewed before any follow-up action.",
            ]),
            MvpPanel::Snippets => panel_lines(&[
                "Snippet parameters are prompted before insertion.",
                "Missing parameters block terminal insertion.",
                "Confirmed snippets insert into the active terminal only.",
            ]),
            MvpPanel::BackgroundJobs => panel_lines(&[
                "Jobs show running, completed, and failed states.",
                "Completion emits a toast and updates the pane badge.",
                "Background reruns stay attached to their command block.",
            ]),
            MvpPanel::CommandBlocks => panel_lines(&[
                "Blocks show confidence, degraded/fallback state, and exit code.",
                "Actions: copy, rerun, explain, attach, save as snippet, background rerun.",
                "Filters and export operate on captured command blocks.",
            ]),
            MvpPanel::Mcp => panel_lines(&[
                "Import JSON, preview command, and review permission summary.",
                "Health, logs, visible process tree, and JobObject caps are shown here.",
                "Secret references are disclosed without rendering secret values.",
            ]),
            MvpPanel::Diagnostics => panel_lines(&[
                "Export lists exact files, logs, process metrics, and OS/hardware metadata.",
                "Command history redaction preview and secret summary are shown before save.",
                "Local save/copy only; upload remains opt-in.",
            ]),
        };

        let card = container(
            column![
                row![
                    text(title).size(20).width(Length::Fill),
                    button(text("Close")).on_press(ShellMessage::ClosePanel)
                ]
                .spacing(8),
                body,
            ]
            .spacing(16),
        )
        .width(Length::Fixed(620.0))
        .padding(16)
        .style(|_theme: &Theme| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgb(
                0.12, 0.12, 0.15,
            ))),
            border: iced::Border {
                color: iced::Color::from_rgb(0.35, 0.35, 0.45),
                width: 1.0,
                radius: 8.0.into(),
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

    fn cmdk_panel_body(&self) -> Element<'_, ShellMessage> {
        let prompt = text_input("Describe the command to prepare...", &self.cmdk_prompt)
            .on_input(ShellMessage::CmdKPromptChanged)
            .on_submit(ShellMessage::CmdKRequestPreview)
            .padding(8);

        let preview = self
            .cmdk_preview
            .as_deref()
            .unwrap_or("No preview generated yet.");

        column![
            text("Preview is generated first; confirm sends it to the terminal."),
            prompt,
            row![
                button(text("Preview")).on_press(ShellMessage::CmdKRequestPreview),
                button(text("Confirm")).on_press(ShellMessage::CmdKConfirmRun),
            ]
            .spacing(8),
            text(format!("Preview: {preview}")).size(13),
        ]
        .spacing(10)
        .into()
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

fn panel_for_command(command: CommandId) -> Option<MvpPanel> {
    match command {
        CommandId::CmdK => Some(MvpPanel::CmdK),
        CommandId::SmartHistory => Some(MvpPanel::SmartHistory),
        CommandId::ExplainLastFailed => Some(MvpPanel::Explainer),
        CommandId::AttachContext | CommandId::OpenCommandBlocks => Some(MvpPanel::CommandBlocks),
        CommandId::ToggleBackgroundJobs => Some(MvpPanel::BackgroundJobs),
        CommandId::OpenSnippets => Some(MvpPanel::Snippets),
        CommandId::OpenMcpPanel => Some(MvpPanel::Mcp),
        CommandId::OpenDiagnostics => Some(MvpPanel::Diagnostics),
        CommandId::OpenCommandPalette
        | CommandId::ReloadSettings
        | CommandId::NewTab
        | CommandId::ClosePane
        | CommandId::SplitPane
        | CommandId::FindInPane
        | CommandId::OpenResourceDashboard => None,
    }
}

fn cmdk_preview_for_prompt(prompt: &str) -> Option<String> {
    let trimmed = prompt.trim();
    if trimmed.is_empty() {
        return None;
    }

    let lower = trimmed.to_ascii_lowercase();
    let command = if lower.contains("git") && lower.contains("status") {
        "git status"
    } else if lower.contains("list") && lower.contains("file") {
        "dir"
    } else if let Some(stripped) = trimmed.strip_prefix('$') {
        stripped.trim()
    } else {
        trimmed
    };

    if command.is_empty() {
        None
    } else {
        Some(command.to_string())
    }
}

fn panel_lines(lines: &[&str]) -> Element<'static, ShellMessage> {
    let mut col = column![].spacing(8);
    for line in lines {
        col = col.push(text((*line).to_string()).size(13));
    }
    col.into()
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
    fn phase3_commands_are_active_after_mvp_ui_wiring() {
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
                CommandAvailability::Active,
                "{id:?} should be active for MVP UI"
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
    fn shell_execute_cmdk_command_opens_cmdk_panel() {
        let mut shell = BongTermShell::default();
        shell.update(ShellMessage::OpenCommandPalette);
        let palette = CommandPalette::default();
        let all = palette.all_commands();
        let cmdk_pos = all.iter().position(|c| c.id == CommandId::CmdK).unwrap();
        for _ in 0..cmdk_pos {
            shell.update(ShellMessage::PaletteSelectNext);
        }
        shell.update(ShellMessage::PaletteExecuteSelected);
        assert!(!shell.command_palette_open());
        assert_eq!(shell.active_panel(), Some(MvpPanel::CmdK));
    }

    #[test]
    fn cmdk_preview_confirm_queues_terminal_command() {
        let mut shell = BongTermShell::default();
        shell.update(ShellMessage::OpenPanel(MvpPanel::CmdK));
        shell.update(ShellMessage::CmdKPromptChanged(
            "show git status".to_string(),
        ));
        shell.update(ShellMessage::CmdKRequestPreview);
        assert_eq!(shell.cmdk_preview(), Some("git status"));
        shell.update(ShellMessage::CmdKConfirmRun);
        assert_eq!(
            shell.take_pending_terminal_command(),
            Some("git status".to_string())
        );
    }

    #[test]
    fn mvp_panels_open_for_history_explainer_jobs_mcp_diagnostics() {
        let mut shell = BongTermShell::default();
        for panel in [
            MvpPanel::SmartHistory,
            MvpPanel::Explainer,
            MvpPanel::Snippets,
            MvpPanel::BackgroundJobs,
            MvpPanel::CommandBlocks,
            MvpPanel::Mcp,
            MvpPanel::Diagnostics,
        ] {
            shell.update(ShellMessage::OpenPanel(panel));
            assert_eq!(shell.active_panel(), Some(panel));
        }
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

    #[test]
    fn resource_row_separates_title_from_metrics() {
        let row = ResourceRowVm {
            category: "BongTerm".to_string(),
            pid: 26_696,
            rss: "15 MB".to_string(),
            cpu_pct: "0.0%".to_string(),
        };

        assert_eq!(row.title_line(), "BongTerm");
        assert_eq!(row.metrics_line(), "pid 26696 | RSS 15 MB | CPU 0.0%");
    }
}
