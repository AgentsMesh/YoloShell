use crate::constants::*;
use yolo_hub_agent_protocol::{Activity, Lifecycle, ObservationSource, RecoveryStatus};

/// A compact writer-control indicator. The glyph carries the meaning; colour
/// only reinforces it. Full wording remains in the details view.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlBadge {
    Writable,
    ReadOnly,
    Paused,
    Syncing,
    Unavailable,
}

impl ControlBadge {
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Writable => CONTROL_WRITABLE_GLYPH,
            Self::ReadOnly => CONTROL_READ_ONLY_GLYPH,
            Self::Paused => CONTROL_PAUSED_GLYPH,
            Self::Syncing => CONTROL_SYNCING_GLYPH,
            Self::Unavailable => CONTROL_UNAVAILABLE_GLYPH,
        }
    }
}

pub(crate) fn spinner_frame(phase: u8) -> &'static str {
    SPINNER_FRAMES[usize::from(phase) % SPINNER_FRAMES.len()]
}

pub(crate) fn lifecycle(value: Lifecycle) -> &'static str {
    match value {
        Lifecycle::Unknown {} => UNKNOWN_LABEL,
        Lifecycle::Starting {} => STARTING_LABEL,
        Lifecycle::Running {} => RUNNING_LABEL,
        Lifecycle::Exited { .. } => EXITED_LABEL,
    }
}

pub(crate) fn activity(value: Activity) -> &'static str {
    match value {
        Activity::Unknown => UNKNOWN_LABEL,
        Activity::Idle => IDLE_LABEL,
        Activity::Working => WORKING_LABEL,
        Activity::WaitingForInput => WAITING_INPUT_LABEL,
        Activity::WaitingForApproval => WAITING_APPROVAL_LABEL,
    }
}

pub(crate) fn reachability(reachable: bool) -> &'static str {
    if reachable {
        REACHABLE_LABEL
    } else {
        UNREACHABLE_LABEL
    }
}

pub(crate) fn observed_reachability(current: bool, reachable: bool) -> &'static str {
    if current {
        reachability(reachable)
    } else {
        UNKNOWN_LABEL
    }
}

pub(crate) fn recovery(value: RecoveryStatus) -> &'static str {
    match value {
        RecoveryStatus::Available => RESUME_AVAILABLE_LABEL,
        RecoveryStatus::Unsupported => RESUME_UNSUPPORTED_LABEL,
        RecoveryStatus::Unavailable => RESUME_UNAVAILABLE_LABEL,
        RecoveryStatus::Unknown => RESUME_UNKNOWN_LABEL,
    }
}

pub(crate) fn source(value: ObservationSource) -> &'static str {
    match value {
        ObservationSource::ProviderHook => HOOK_SOURCE_LABEL,
        ObservationSource::ProcessObserver => PROCESS_SOURCE_LABEL,
        ObservationSource::Connector => CONNECTOR_SOURCE_LABEL,
    }
}
