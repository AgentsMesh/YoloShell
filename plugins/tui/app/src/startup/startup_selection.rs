use crate::{State, startup_constants::*};
use yolo_hub_identity::{HubId, TerminalAddress};
use yolo_hub_terminal_protocol::{RuntimeStatus, TerminalSummary};

pub(crate) fn live(session: &TerminalSummary) -> bool {
    !session.fields().archived
        && session.fields().runtime.is_some_and(|runtime| {
            matches!(
                runtime.status,
                RuntimeStatus::Running | RuntimeStatus::Starting
            )
        })
}

pub(crate) fn attach(
    state: &State,
    hub: HubId,
    selector: Option<&str>,
) -> Result<TerminalAddress, &'static str> {
    if selector.is_none() {
        if let Some(address) = state.focused_terminal() {
            return state
                .terminals
                .contains_key(&address)
                .then_some(address)
                .ok_or(NO_SESSION);
        }
    }
    let mut matches = state
        .terminals
        .values()
        .filter(|session| {
            if session.fields().archived {
                return false;
            }
            match selector {
                Some(selector) => session.matches_selector(selector),
                None => session.fields().address.hub_id == hub && live(session),
            }
        })
        .map(|session| session.fields().address);
    let address = matches.next().ok_or(NO_SESSION)?;
    if matches.next().is_some() {
        Err(AMBIGUOUS_SESSION)
    } else {
        Ok(address)
    }
}
