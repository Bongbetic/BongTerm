//! Agent sidebar view-model + Iced view. UI-owned DTOs only — this module
//! must not depend on `bongterm-agents` (see allowed-deps.toml). The app
//! layer translates agent-domain state into these plain DTOs.

use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

use crate::ShellMessage;

/// UI-local mirror of agent lifecycle state (no domain dependency).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatusVm {
    Idle,
    Running,
    Stopping,
    Exited,
    Killed,
    Crashed,
}

impl AgentStatusVm {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Exited => "exited",
            Self::Killed => "killed",
            Self::Crashed => "crashed",
        }
    }
}

/// A lifecycle control button the sidebar can offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleControl {
    Stop,
    KillTree,
    Restart,
}

impl LifecycleControl {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Stop => "Stop",
            Self::KillTree => "Kill tree",
            Self::Restart => "Restart",
        }
    }
}

/// Which lifecycle controls are legal for a given status.
#[must_use]
pub fn available_controls(status: AgentStatusVm) -> Vec<LifecycleControl> {
    match status {
        AgentStatusVm::Running => vec![LifecycleControl::Stop, LifecycleControl::KillTree],
        AgentStatusVm::Stopping => vec![LifecycleControl::KillTree],
        AgentStatusVm::Exited | AgentStatusVm::Killed | AgentStatusVm::Crashed => {
            vec![LifecycleControl::Restart]
        }
        AgentStatusVm::Idle => vec![],
    }
}

/// One agent row in the sidebar.
#[derive(Debug, Clone)]
pub struct AgentRowVm {
    pub run_id: String,
    pub name: String,
    pub status: AgentStatusVm,
    /// Mid-session steering is only offered when the adapter exposes a
    /// supported control channel. Never simulated.
    pub steering_available: bool,
}

/// One pending-approval row in the sidebar.
#[derive(Debug, Clone)]
pub struct ApprovalRowVm {
    pub approval_id: u64,
    pub action: String,
    pub reason: String,
    /// Display string from enforcement policy (e.g. "require-approval", "deny").
    pub enforcement_label: String,
}

/// Whole-sidebar view-model.
#[derive(Debug, Clone)]
pub struct AgentSidebarVm {
    pub agents: Vec<AgentRowVm>,
    pub approvals: Vec<ApprovalRowVm>,
}

impl AgentSidebarVm {
    /// Build the Iced element for the sidebar.
    #[must_use]
    pub fn view(&self) -> Element<'_, ShellMessage> {
        let mut col = column![text("Agents").size(16)].spacing(8);

        if self.agents.is_empty() {
            col = col.push(text("No agents running").size(12));
        } else {
            for a in &self.agents {
                let mut controls =
                    row![text(format!("{} [{}]", a.name, a.status.label()))].spacing(6);
                for ctrl in available_controls(a.status) {
                    controls = controls.push(button(text(ctrl.label()).size(12)).on_press(
                        ShellMessage::AgentLifecycle {
                            run_id: a.run_id.clone(),
                            control: ctrl,
                        },
                    ));
                }
                if a.steering_available {
                    controls = controls.push(button(text("Interrupt").size(12)).on_press(
                        ShellMessage::AgentInterrupt {
                            run_id: a.run_id.clone(),
                        },
                    ));
                }
                col = col.push(controls);
            }
        }

        col = col.push(text("Approvals").size(16));
        if self.approvals.is_empty() {
            col = col.push(text("No pending approvals").size(12));
        } else {
            for ap in &self.approvals {
                let r = row![
                    text(format!("{} — {}", ap.action, ap.reason)).width(Length::Fill),
                    text(&ap.enforcement_label).size(12),
                    button(text("Approve").size(12)).on_press(ShellMessage::ApprovalResolve {
                        approval_id: ap.approval_id,
                        approve: true
                    }),
                    button(text("Reject").size(12)).on_press(ShellMessage::ApprovalResolve {
                        approval_id: ap.approval_id,
                        approve: false
                    }),
                ]
                .spacing(6);
                col = col.push(r);
            }
        }

        container(col)
            .width(220)
            .height(Length::Fill)
            .padding(8)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vm() -> AgentSidebarVm {
        AgentSidebarVm {
            agents: vec![
                AgentRowVm {
                    run_id: "run-1".to_string(),
                    name: "claude-code".to_string(),
                    status: AgentStatusVm::Running,
                    steering_available: false,
                },
                AgentRowVm {
                    run_id: "run-2".to_string(),
                    name: "codex-cli".to_string(),
                    status: AgentStatusVm::Crashed,
                    steering_available: false,
                },
            ],
            approvals: vec![ApprovalRowVm {
                approval_id: 7,
                action: "git push --force".to_string(),
                reason: "destructive".to_string(),
                enforcement_label: "require-approval".to_string(),
            }],
        }
    }

    #[test]
    fn running_agent_allows_stop_and_kill_not_restart() {
        let row = &vm().agents[0];
        let controls = available_controls(row.status);
        assert!(controls.contains(&LifecycleControl::Stop));
        assert!(controls.contains(&LifecycleControl::KillTree));
        assert!(!controls.contains(&LifecycleControl::Restart));
    }

    #[test]
    fn crashed_agent_allows_restart_only() {
        let row = &vm().agents[1];
        let controls = available_controls(row.status);
        assert_eq!(controls, vec![LifecycleControl::Restart]);
    }

    #[test]
    fn approval_row_exposes_enforcement_label_text() {
        let v = vm();
        assert_eq!(v.approvals[0].enforcement_label, "require-approval");
    }

    #[test]
    fn view_builds_without_panicking() {
        let v = vm();
        let _element = v.view();
    }

    #[test]
    fn empty_sidebar_renders_placeholder() {
        let v = AgentSidebarVm {
            agents: vec![],
            approvals: vec![],
        };
        let _element = v.view();
        assert!(v.agents.is_empty());
    }
}
