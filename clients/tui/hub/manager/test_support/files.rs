#[path = "file_constants.rs"]
pub mod constants;
use super::fixture::*;
use crate::{
    Manager,
    fields::FieldId,
    model::{Action, Page},
};
use yolo_hub_file_protocol::FileRootConfiguration;

pub fn add(manager: &mut Manager) -> FileRootConfiguration {
    manager.page(Page::FileRoots);
    activate(manager, Action::AddFileRoot);
    field(manager, FieldId::RootName, constants::NAME);
    field(manager, FieldId::RootPath, constants::PATH);
    save(manager);
    manager
        .draft
        .as_ref()
        .unwrap()
        .file_roots
        .last()
        .unwrap()
        .clone()
}
