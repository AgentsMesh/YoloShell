use crate::{MAX_DOCUMENT_PANES, MAX_TABS, MAX_TITLE_BYTES, SharedTab, ProtocolError, identity};
use std::collections::BTreeSet;

pub(crate) fn shared_tab(tab: &SharedTab) -> Result<(), ProtocolError> {
    identity::require(tab.id)?;
    if tab.title.is_empty()
        || tab.title.len() > MAX_TITLE_BYTES
        || tab.title.trim() != tab.title
        || tab.title.chars().any(char::is_control)
    {
        return Err(ProtocolError::InvalidTitle);
    }
    tab
        .root
        .validate()
        .map_err(|_| ProtocolError::InvalidTree)?;
    Ok(())
}

pub(crate) fn tabs(tabs: &[SharedTab]) -> Result<(), ProtocolError> {
    if tabs.len() > MAX_TABS {
        return Err(ProtocolError::Limit);
    }
    let mut ids = BTreeSet::new();
    let mut count = 0;
    for value in tabs {
        shared_tab(value)?;
        if !ids.insert(value.id) {
            return Err(ProtocolError::DuplicateId);
        }
        for pane in value.root.panes() {
            if !ids.insert(pane.id) {
                return Err(ProtocolError::DuplicateId);
            }
            count += 1;
        }
    }
    if count > MAX_DOCUMENT_PANES {
        return Err(ProtocolError::Limit);
    }
    Ok(())
}
