use crate::{
    FIRST_REVISION, LayoutAddress, SharedLayoutDocument, MAX_DOCUMENT_PANES, MAX_TABS, ProtocolError,
    identity,
};
use serde::{Deserialize, Serialize};

/// A bounded directory entry. Fetch the document separately to inspect its topology.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SummaryFields")]
pub struct LayoutSummary {
    pub address: LayoutAddress,
    pub revision: u64,
    pub tab_count: u32,
    pub pane_count: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SummaryFields {
    address: LayoutAddress,
    revision: u64,
    tab_count: u32,
    pane_count: u32,
}

impl LayoutSummary {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        identity::require(self.address.layout_id)?;
        if self.revision < FIRST_REVISION {
            return Err(ProtocolError::InvalidRevision);
        }
        let tabs = self.tab_count as usize;
        let panes = self.pane_count as usize;
        if tabs > MAX_TABS
            || panes > MAX_DOCUMENT_PANES
            || panes < tabs
            || panes > tabs * yolo_presentation_layout::MAX_PANES
        {
            return Err(ProtocolError::Limit);
        }
        Ok(())
    }
}

impl TryFrom<&SharedLayoutDocument> for LayoutSummary {
    type Error = ProtocolError;
    fn try_from(document: &SharedLayoutDocument) -> Result<Self, Self::Error> {
        document.validate()?;
        Ok(Self {
            address: document.address,
            revision: document.revision,
            // Validated public limits are smaller than u32 on every supported platform.
            tab_count: document.tabs.len() as u32,
            pane_count: document
                .tabs
                .iter()
                .map(|tab| tab.root.panes().len())
                .sum::<usize>() as u32,
        })
    }
}

impl TryFrom<SummaryFields> for LayoutSummary {
    type Error = ProtocolError;
    fn try_from(fields: SummaryFields) -> Result<Self, Self::Error> {
        let summary = Self {
            address: fields.address,
            revision: fields.revision,
            tab_count: fields.tab_count,
            pane_count: fields.pane_count,
        };
        summary.validate()?;
        Ok(summary)
    }
}
