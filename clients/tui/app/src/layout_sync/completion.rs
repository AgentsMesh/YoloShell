use super::{Operation, Sync, adopt, constants::*};
use crate::{State, constants::CATALOG_REFRESH};
use std::time::Instant;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_presentation_protocol::SharedLayoutDocument;

pub(crate) fn complete(state: &mut State, operation: Operation, result: Result<Reply, String>) {
    let Some(mut sync) = state.shared_layout.take() else {
        return;
    };
    if sync.pending.as_ref().is_none_or(|pending| {
        pending.token != operation.token || pending.command != operation.command
    }) {
        state.shared_layout = Some(sync);
        return;
    }
    sync.pending = None;
    let result = result.and_then(|reply| match reply {
        Reply::Layout {
            document: Some(document),
        } => reconcile(state, &mut sync, &operation.command, document),
        Reply::Layout { document: None } => Err(MISSING.into()),
        _ => Err(INVALID.into()),
    });
    if let Err(message) = result {
        if let Command::UpdateLayout { change } = operation.command {
            sync.uncertain = Some(change);
        }
        sync.read = true;
        sync.retry = Instant::now() + CATALOG_REFRESH;
        state.status = message;
    }
    state.shared_layout = Some(sync);
}

fn reconcile(
    state: &mut State,
    sync: &mut Sync,
    command: &Command,
    document: SharedLayoutDocument,
) -> Result<(), String> {
    if document.validate().is_err()
        || document.address != sync.document.address
        || document.revision < sync.document.revision
    {
        return Err(INVALID.into());
    }
    let submitted = match command {
        Command::UpdateLayout { change } => Some(change),
        Command::GetLayout { .. } => sync.uncertain.as_ref(),
        _ => return Err(INVALID.into()),
    };
    let own = submitted.is_some_and(|change| {
        change
            .apply(Some(&sync.document))
            .is_ok_and(|expected| expected == document)
    });
    if matches!(command, Command::UpdateLayout { .. }) && !own {
        return Err(INVALID.into());
    }
    if own {
        // The current UI may include edits made after this operation was sent.
        // Keep that draft and advance its CAS base only after proving this exact write.
        sync.projected = document.tabs.clone();
    } else if document.revision == sync.document.revision {
        if document != sync.document {
            return Err(INVALID.into());
        }
    } else {
        let dirty = state
            .tab_snapshot()
            .shared_tabs(state.entry_hub)
            .map_err(|error| error.to_string())?
            != sync.projected;
        sync.projected = adopt(state, &document).map_err(|error| error.to_string())?;
        if dirty {
            state.status = CONFLICT.into();
        }
    }
    sync.document = document;
    sync.uncertain = None;
    sync.read = false;
    sync.retry = Instant::now();
    Ok(())
}
