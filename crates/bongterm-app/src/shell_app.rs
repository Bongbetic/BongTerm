//! Composition root for the visible BongTerm application.
//!
//! `bongterm-ui` owns shell chrome and presentation state. `terminal_app` owns
//! the live ConPTY/VT/render path. This module is the app-layer boundary that
//! places the live terminal surface inside the shell chrome.

use std::sync::Arc;

use bongterm_ledger::{
    CurrentProcessSampler, DashboardViewModel, DxgiVramSampler, ResourceSampler,
};
use bongterm_ui::{
    BongTermShell, ResourceDashboardVm, ResourceRowVm, ShellMessage, TerminalSurfaceSize,
    agent_sidebar::AgentSidebarVm,
};
use iced::event::{self, Event};
use iced::{Element, Subscription, Task, Theme};

use crate::terminal_app::{self, TerminalApp};

pub struct BongTermApp {
    shell: BongTermShell,
    terminal: TerminalApp,
}

#[derive(Debug, Clone)]
pub enum AppMessage {
    Shell(ShellMessage),
    Terminal(terminal_app::Message),
    WindowResized(f32, f32),
}

impl BongTermApp {
    pub fn boot() -> (Self, Task<AppMessage>) {
        let (mut shell, shell_task) = BongTermShell::boot();
        let (terminal, terminal_task) = TerminalApp::boot();
        shell.set_panel_data(AgentSidebarVm::default(), sample_resource_dashboard());

        (
            Self { shell, terminal },
            Task::batch([
                shell_task.map(AppMessage::Shell),
                terminal_task.map(AppMessage::Terminal),
            ]),
        )
    }

    pub fn update(&mut self, message: AppMessage) -> Task<AppMessage> {
        match message {
            AppMessage::Shell(message) => self.shell.update(message).map(AppMessage::Shell),
            AppMessage::Terminal(message) => {
                self.terminal.update(message).map(AppMessage::Terminal)
            }
            AppMessage::WindowResized(width, height) => {
                let surface = Self::terminal_surface_size_for_window(width, height);
                self.terminal
                    .update(terminal_app::Message::SurfaceResized(
                        surface.width,
                        surface.height,
                    ))
                    .map(AppMessage::Terminal)
            }
        }
    }

    pub fn subscription(&self) -> Subscription<AppMessage> {
        Subscription::batch([
            self.shell.subscription().map(AppMessage::Shell),
            self.terminal
                .subscription_without_resize()
                .map(AppMessage::Terminal),
            event::listen_raw(|raw, _status, _window| match raw {
                Event::Window(iced::window::Event::Resized(size)) => {
                    Some(AppMessage::WindowResized(size.width, size.height))
                }
                _ => None,
            }),
        ])
    }

    pub fn view(&self) -> Element<'_, AppMessage> {
        let terminal = self.terminal.view().map(AppMessage::Terminal);
        self.shell.view_with_terminal(terminal, AppMessage::Shell)
    }

    #[must_use]
    pub fn title(&self) -> String {
        self.shell.title()
    }

    #[must_use]
    pub const fn theme(&self) -> Theme {
        Theme::Dark
    }

    #[must_use]
    pub const fn shell_region_names(&self) -> [&'static str; 7] {
        self.shell.region_names()
    }

    #[must_use]
    pub const fn terminal_grid_size(&self) -> (u16, u16) {
        self.terminal.grid_size()
    }

    #[must_use]
    pub fn terminal_surface_size_for_window(width: f32, height: f32) -> TerminalSurfaceSize {
        BongTermShell::terminal_surface_size_for_window(width, height)
    }

    #[must_use]
    pub fn agent_sidebar_snapshot(&self) -> &AgentSidebarVm {
        self.shell.agent_sidebar_snapshot()
    }

    #[must_use]
    pub const fn resource_dashboard_snapshot(&self) -> &ResourceDashboardVm {
        self.shell.resource_dashboard_snapshot()
    }
}

fn sample_resource_dashboard() -> ResourceDashboardVm {
    let sampler = CurrentProcessSampler::new(Arc::new(DxgiVramSampler));
    let sample = sampler.take_sample();
    let dashboard = DashboardViewModel::from_sample(&sample);
    resource_dashboard_from_ledger(dashboard)
}

fn resource_dashboard_from_ledger(dashboard: DashboardViewModel) -> ResourceDashboardVm {
    ResourceDashboardVm {
        total_rss: dashboard.total_rss,
        total_cpu_pct: dashboard.total_cpu_pct,
        vram: dashboard.vram,
        rows: dashboard
            .rows
            .into_iter()
            .map(|row| ResourceRowVm {
                category: row.category,
                pid: row.pid,
                rss: row.rss,
                cpu_pct: row.cpu_pct,
            })
            .collect(),
        is_stale: dashboard.is_stale,
    }
}

/// Launch the composed BongTerm window.
///
/// # Errors
/// Returns an error if the iced runtime fails to start.
pub fn run() -> iced::Result {
    iced::application(BongTermApp::boot, BongTermApp::update, BongTermApp::view)
        .title(BongTermApp::title)
        .theme(BongTermApp::theme)
        .subscription(BongTermApp::subscription)
        .run()
}
