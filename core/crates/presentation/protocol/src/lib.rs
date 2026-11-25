//! Client-neutral SharedTab/Pane layout documents. No process or writer authority lives here.
mod address;
mod constants;
mod shared_document;
mod error;
mod identity;
mod shared_tab;
mod summary;
mod update;
mod validation;

pub use address::LayoutAddress;
pub use constants::{
    FIRST_REVISION, MAX_DOCUMENT_PANES, MAX_DOCUMENTS, MAX_TABS, MAX_TITLE_BYTES,
};
pub use shared_document::SharedLayoutDocument;
pub use error::ProtocolError;
pub use shared_tab::SharedTab;
pub use summary::LayoutSummary;
pub use update::LayoutUpdate;

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
