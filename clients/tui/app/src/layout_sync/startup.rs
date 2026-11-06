use super::{Sync, adopt, constants::*, network::call};
use crate::{Options, State};
use anyhow::{Result, anyhow, ensure};
use uuid::Uuid;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_presentation_protocol::LayoutUpdate;
use yolo_tui_tab::TabStore;

pub(crate) async fn open(options: &Options, store: &mut TabStore, state: &mut State) -> Result<()> {
    let mut address = store.document().layout_address;
    if let Some(id) = options.layout {
        ensure!(!id.is_nil(), INVALID);
        address.layout_id = id;
    }
    let reply = call(options, Command::GetLayout { address })
        .await
        .map_err(|error| anyhow!(error))?;
    let Reply::Layout { document } = reply else {
        return Err(anyhow!(INVALID));
    };
    let document = match document {
        Some(document) => document,
        None => {
            ensure!(options.layout.is_none(), MISSING);
            let change = LayoutUpdate {
                address,
                request_id: Uuid::new_v4(),
                expected_revision: 0,
                tabs: state.tab_snapshot().shared_tabs(state.entry_hub)?,
            };
            let expected = change.apply(None)?;
            let reply = call(options, Command::UpdateLayout { change })
                .await
                .map_err(|error| anyhow!(error))?;
            let Reply::Layout {
                document: Some(document),
            } = reply
            else {
                return Err(anyhow!(INVALID));
            };
            ensure!(document == expected, INVALID);
            document
        }
    };
    ensure!(document.address == address, INVALID);
    document.validate()?;
    let projected = if options.layout.is_none() {
        restore_draft(state, &document, store.document().shared.as_ref())?
    } else {
        adopt(state, &document)?
    };
    if options.layout.is_some() {
        store.select_layout(address.layout_id)?;
    }
    state.shared_layout = Some(Sync::new(document, projected));
    Ok(())
}

pub(crate) fn restore_draft(
    state: &mut State,
    document: &yolo_presentation_protocol::SharedLayoutDocument,
    cached: Option<&yolo_tui_tab::LayoutCheckpoint>,
) -> Result<Vec<yolo_presentation_protocol::SharedTab>> {
    if let Some(cached) = cached {
        let draft = state.tab_snapshot().shared_tabs(state.entry_hub)?;
        if draft != cached.projected {
            if *document == cached.document {
                return Ok(cached.projected.clone());
            }
            ensure!(document.tabs == draft, DIRTY_RESTART);
        }
    }
    adopt(state, document)
}
