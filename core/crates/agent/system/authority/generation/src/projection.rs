use yolo_agent_contract::AgentState;
use yolo_hooks_runtime::{
    AttentionState, InstrumentationState, NormalizedLifecycle, ProcessState, TurnState,
};

/// Provider-specific differences around process evidence and turn endings.
/// Everything else (permission/user-input attention, idle/running turns) is
/// shared so provider projections cannot drift.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessRule {
    FallThrough,
    RequireAlive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitRule {
    Any,
    FinalHook,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StopRule {
    Running,
    ActiveIfPending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnknownRule {
    Unknown,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectionRules {
    pub require_authoritative: bool,
    pub process_unknown: ProcessRule,
    pub exit: ExitRule,
    pub stop: StopRule,
    pub turn_unknown: UnknownRule,
}

impl ProjectionRules {
    /// Generic hook-generation authority: any observed exit, StopCandidate is
    /// still running, and unknown turns publish Unknown.
    pub const GENERATION: Self = Self {
        require_authoritative: false,
        process_unknown: ProcessRule::FallThrough,
        exit: ExitRule::Any,
        stop: StopRule::Running,
        turn_unknown: UnknownRule::Unknown,
    };

    /// Codex occurrence witness: only authoritative evidence, an exit needs
    /// the final hook, and a Stop candidate is Active only while pending.
    pub const CODEX: Self = Self {
        require_authoritative: true,
        process_unknown: ProcessRule::RequireAlive,
        exit: ExitRule::FinalHook,
        stop: StopRule::ActiveIfPending,
        turn_unknown: UnknownRule::None,
    };
}

pub fn project(lifecycle: &NormalizedLifecycle, rules: ProjectionRules) -> Option<AgentState> {
    if rules.require_authoritative
        && lifecycle.instrumentation != InstrumentationState::Authoritative
    {
        return None;
    }
    match lifecycle.process {
        ProcessState::Exited => {
            return (rules.exit == ExitRule::Any || lifecycle.final_hook_seen)
                .then_some(AgentState::Exited);
        }
        ProcessState::Unknown if rules.process_unknown == ProcessRule::RequireAlive => return None,
        ProcessState::Alive | ProcessState::Unknown => {}
    }
    match lifecycle.attention {
        AttentionState::Permission | AttentionState::PermissionCandidate => {
            return Some(AgentState::WaitingForPermission);
        }
        AttentionState::UserInput | AttentionState::Elicitation => {
            return Some(AgentState::WaitingForInput);
        }
        AttentionState::None => {}
    }
    match lifecycle.turn {
        TurnState::Idle => Some(AgentState::WaitingForInput),
        TurnState::Running | TurnState::Compacting => Some(AgentState::Running),
        TurnState::StopCandidate => match rules.stop {
            StopRule::Running => Some(AgentState::Running),
            StopRule::ActiveIfPending => {
                lifecycle.stop_candidate.as_ref().map(|_| AgentState::Active)
            }
        },
        TurnState::Failed | TurnState::Unknown => match rules.turn_unknown {
            UnknownRule::Unknown => Some(AgentState::Unknown),
            UnknownRule::None => None,
        },
    }
}
