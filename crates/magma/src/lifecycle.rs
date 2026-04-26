//! Lifecycle: explicit state machine for a Process.
//!
//! Lava encodes lifecycle implicitly via `MGMT_COMMAND` / `MGMT_RESPONSE`
//! enums passed over CSP channels. We mirror that exactly, but also expose
//! a typed `ProcessState` so the runtime can check legal transitions.

use std::fmt;

/// Commands the Runtime sends to a RuntimeService and onwards to a Process.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MgmtCommand {
    /// Run for `n` ticks; `0` means run continuously.
    Run { steps: u64 },
    /// Pause at the next safe boundary (after the current phase completes).
    Pause,
    /// Stop and tear down.
    Stop,
    /// Read a Var by id (handled at runtime layer, surfaced here for symmetry).
    GetData,
    /// Write a Var by id.
    SetData,
}

/// Responses a Process sends back to its RuntimeService.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum MgmtResponse {
    Done,
    Paused,
    Terminated,
    Error,
    ReqStop,
    ReqPause,
}

/// Typed lifecycle state. Transitions are validated by the runtime.
///
/// ```text
///   Initing ─▶ Compiled ─▶ Running ⇄ Paused ─▶ Stopped
///                                     │
///                                     └──────▶ Terminated
/// ```
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ProcessState {
    Initing,
    Compiled,
    Running,
    Paused,
    Stopped,
    Terminated,
    Error,
}

impl ProcessState {
    /// Returns true iff `self -> next` is a legal transition.
    pub fn can_transition_to(self, next: ProcessState) -> bool {
        use ProcessState::*;
        matches!(
            (self, next),
            (Initing, Compiled)
                | (Compiled, Running)
                | (Running, Paused)
                | (Running, Stopped)
                | (Running, Terminated)
                | (Running, Error)
                | (Paused, Running)
                | (Paused, Stopped)
                | (Paused, Terminated)
                | (_, Error)
        )
    }
}

impl fmt::Display for ProcessState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Initing => "initing",
            Self::Compiled => "compiled",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Stopped => "stopped",
            Self::Terminated => "terminated",
            Self::Error => "error",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_transitions() {
        assert!(ProcessState::Initing.can_transition_to(ProcessState::Compiled));
        assert!(ProcessState::Compiled.can_transition_to(ProcessState::Running));
        assert!(ProcessState::Running.can_transition_to(ProcessState::Paused));
        assert!(ProcessState::Paused.can_transition_to(ProcessState::Running));
        assert!(ProcessState::Running.can_transition_to(ProcessState::Stopped));
    }

    #[test]
    fn illegal_transitions() {
        assert!(!ProcessState::Initing.can_transition_to(ProcessState::Running));
        assert!(!ProcessState::Stopped.can_transition_to(ProcessState::Running));
        assert!(!ProcessState::Terminated.can_transition_to(ProcessState::Running));
    }

    #[test]
    fn any_state_can_transition_to_error() {
        for s in [
            ProcessState::Initing,
            ProcessState::Compiled,
            ProcessState::Running,
            ProcessState::Paused,
        ] {
            assert!(s.can_transition_to(ProcessState::Error));
        }
    }
}
