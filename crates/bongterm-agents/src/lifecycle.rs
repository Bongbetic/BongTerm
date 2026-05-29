//! Agent lifecycle state machine. Bounded, closed enum + exhaustive match
//! (SOLID-in-Rust: no dynamic dispatch where the set is fixed). The actual
//! OS process-tree kill is performed by `bongterm-process-control`; this type
//! only models legal state.

/// Observable lifecycle state of an agent run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Idle,
    Running,
    Stopping,
    Exited,
    Killed,
    Crashed,
}

/// Commands that drive lifecycle transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleCommand {
    Launch,
    Stop,
    KillTree,
    Restart,
    ProcessExited,
    Crashed,
}

/// Error returned for an illegal transition.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("illegal lifecycle transition: {command:?} from {state:?}")]
pub struct IllegalTransition {
    pub state: LifecycleState,
    pub command: LifecycleCommand,
}

/// Owns the current lifecycle state and enforces legal transitions.
#[derive(Debug, Clone)]
pub struct AgentLifecycle {
    state: LifecycleState,
}

impl Default for AgentLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentLifecycle {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: LifecycleState::Idle,
        }
    }

    #[must_use]
    pub fn state(&self) -> LifecycleState {
        self.state
    }

    /// Apply a command; update state or reject as illegal.
    pub fn apply(&mut self, command: LifecycleCommand) -> Result<LifecycleState, IllegalTransition> {
        use LifecycleCommand as C;
        use LifecycleState as S;
        let next = match (self.state, command) {
            (S::Idle, C::Launch) => S::Running,
            (S::Running, C::Stop) => S::Stopping,
            (S::Running, C::KillTree) => S::Killed,
            (S::Running, C::ProcessExited) => S::Exited,
            (S::Running, C::Crashed) => S::Crashed,
            (S::Stopping, C::ProcessExited) => S::Exited,
            (S::Stopping, C::KillTree) => S::Killed,
            (S::Exited | S::Killed | S::Crashed, C::Restart) => S::Running,
            (state, command) => return Err(IllegalTransition { state, command }),
        };
        self.state = next;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_lifecycle_starts_idle() {
        assert_eq!(AgentLifecycle::new().state(), LifecycleState::Idle);
    }

    #[test]
    fn launch_moves_idle_to_running() {
        let mut lc = AgentLifecycle::new();
        assert!(lc.apply(LifecycleCommand::Launch).is_ok());
        assert_eq!(lc.state(), LifecycleState::Running);
    }

    #[test]
    fn stop_moves_running_to_stopping_then_exited() {
        let mut lc = AgentLifecycle::new();
        lc.apply(LifecycleCommand::Launch).unwrap();
        lc.apply(LifecycleCommand::Stop).unwrap();
        assert_eq!(lc.state(), LifecycleState::Stopping);
        lc.apply(LifecycleCommand::ProcessExited).unwrap();
        assert_eq!(lc.state(), LifecycleState::Exited);
    }

    #[test]
    fn kill_tree_from_running_goes_to_killed() {
        let mut lc = AgentLifecycle::new();
        lc.apply(LifecycleCommand::Launch).unwrap();
        lc.apply(LifecycleCommand::KillTree).unwrap();
        assert_eq!(lc.state(), LifecycleState::Killed);
    }

    #[test]
    fn restart_from_exited_returns_to_running() {
        let mut lc = AgentLifecycle::new();
        lc.apply(LifecycleCommand::Launch).unwrap();
        lc.apply(LifecycleCommand::ProcessExited).unwrap();
        assert_eq!(lc.state(), LifecycleState::Exited);
        lc.apply(LifecycleCommand::Restart).unwrap();
        assert_eq!(lc.state(), LifecycleState::Running);
    }

    #[test]
    fn illegal_transition_is_rejected() {
        let mut lc = AgentLifecycle::new();
        // Cannot Stop while Idle.
        assert!(lc.apply(LifecycleCommand::Stop).is_err());
        assert_eq!(lc.state(), LifecycleState::Idle);
    }

    #[test]
    fn crash_from_running_marks_crashed_and_allows_restart() {
        let mut lc = AgentLifecycle::new();
        lc.apply(LifecycleCommand::Launch).unwrap();
        lc.apply(LifecycleCommand::Crashed).unwrap();
        assert_eq!(lc.state(), LifecycleState::Crashed);
        lc.apply(LifecycleCommand::Restart).unwrap();
        assert_eq!(lc.state(), LifecycleState::Running);
    }
}
