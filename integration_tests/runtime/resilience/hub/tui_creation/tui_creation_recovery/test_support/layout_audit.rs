use std::path::Path;
use yolo_hub_rpc_protocol::{Command, Reply, Request};
use yolo_tui_tab::{TAB_FILE, TabDocument};

#[derive(Debug)]
pub(super) struct InvalidLayout;

/// Shared layout initialization is independent of terminal creation. Admit only
/// this client's unbound presentation; it cannot attach a terminal or gain input.
pub(super) fn request(request: &Request, workspace: &Path) -> Result<(), InvalidLayout> {
    let Command::UpdateLayout { change } = &request.fields().command else {
        return Err(InvalidLayout);
    };
    let current: TabDocument =
        yolo_paths::read_json(&workspace.join(TAB_FILE)).map_err(|_| InvalidLayout)?;
    current
        .validate(request.fields().destination)
        .map_err(|_| InvalidLayout)?;
    change.validate().map_err(|_| InvalidLayout)?;
    if change.address != current.layout_address
        || change.tabs.iter().any(|tab| {
            tab
                .root
                .panes()
                .iter()
                .any(|pane| pane.terminal.is_some())
        })
    {
        return Err(InvalidLayout);
    }
    Ok(())
}

pub(super) fn reply(request: &Request, response: &Reply) -> Result<(), InvalidLayout> {
    let Command::UpdateLayout { change } = &request.fields().command else {
        return Ok(());
    };
    let Reply::Layout {
        document: Some(document),
    } = response
    else {
        return Err(InvalidLayout);
    };
    document.validate().map_err(|_| InvalidLayout)?;
    if document.address != change.address
        || document.last_request_id != change.request_id
        || document.tabs != change.tabs
        || change
            .expected_revision
            .checked_add(yolo_presentation_protocol::FIRST_REVISION)
            != Some(document.revision)
    {
        return Err(InvalidLayout);
    }
    Ok(())
}
