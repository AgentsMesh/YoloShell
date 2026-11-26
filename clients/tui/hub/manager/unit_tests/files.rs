use super::{
    files_support::{self, constants::*},
    fixture::*,
};
use crate::{
    Effect,
    constants::*,
    fields::{FieldId, Value},
    model::{Action, Page},
};
use crossterm::event::KeyCode;

#[test]
fn roots_keep_their_generated_identity_and_exact_path_and_only_apply_sends_a_request() {
    let mut manager = manager();
    manager.page(Page::FileRoots);
    activate(&mut manager, Action::AddFileRoot);
    let form = manager.form.as_ref().unwrap();
    let original = crate::wire_fields::text(&form.fields, FieldId::RootId)
        .unwrap()
        .to_owned();
    assert!(
        form.fields
            .iter()
            .find(|field| field.id == FieldId::RootId)
            .unwrap()
            .read_only
    );
    field(&mut manager, FieldId::RootId, CHANGED_PATH);
    field(&mut manager, FieldId::RootName, NAME);
    field(&mut manager, FieldId::RootPath, PATH);
    save(&mut manager);
    let root = manager.draft.as_ref().unwrap().file_roots[0].clone();
    assert_eq!(root.fields().root_id.to_string(), original);
    assert_eq!(root.fields().path, PATH);
    assert!(
        manager
            .base
            .as_ref()
            .unwrap()
            .state
            .configuration
            .file_roots
            .is_empty()
    );
    assert!(!manager.pending());
    manager.page(Page::Root);
    let Some(Effect::Request(operation)) = activate(&mut manager, Action::Apply) else {
        panic!()
    };
    let yolo_hub_rpc_protocol::Command::Configure { change } = operation.command else {
        panic!()
    };
    assert_eq!(change.configuration.file_roots, vec![root]);
}

#[test]
fn invalid_paths_can_be_corrected_without_creating_another_identity() {
    let mut manager = manager();
    manager.page(Page::FileRoots);
    activate(&mut manager, Action::AddFileRoot);
    field(&mut manager, FieldId::RootName, NAME);
    field(&mut manager, FieldId::RootPath, INVALID_PATH);
    let id = crate::wire_fields::text(&manager.form.as_ref().unwrap().fields, FieldId::RootId)
        .unwrap()
        .to_owned();
    save(&mut manager);
    assert!(manager.form.is_some());
    assert!(!manager.dirty());
    field(&mut manager, FieldId::RootPath, PATH);
    save(&mut manager);
    assert_eq!(
        manager.draft.as_ref().unwrap().file_roots[0]
            .fields()
            .root_id
            .to_string(),
        id
    );
}

#[test]
fn editing_and_removal_preserve_other_configuration_and_require_confirmation() {
    let mut manager = manager();
    let root = files_support::add(&mut manager);
    let peers = manager.draft.as_ref().unwrap().peers.len();
    activate(&mut manager, Action::EditFileRoot(root.fields().root_id));
    field(&mut manager, FieldId::RootPath, CHANGED_PATH);
    save(&mut manager);
    let edited = manager.draft.as_ref().unwrap().file_roots[0].clone();
    assert_eq!(edited.fields().root_id, root.fields().root_id);
    assert_eq!(edited.fields().path, CHANGED_PATH);
    activate(&mut manager, Action::EditFileRoot(root.fields().root_id));
    let form = manager.form.as_mut().unwrap();
    form.focused = form.rows() - STEP;
    key(&mut manager, KeyCode::Enter);
    assert_eq!(
        manager.confirmation.as_ref().unwrap().label,
        REMOVE_FILE_ROOT
    );
    key(&mut manager, KeyCode::Enter);
    assert_eq!(
        manager.draft.as_ref().unwrap().file_roots,
        vec![edited.clone()]
    );
    key(&mut manager, KeyCode::Enter);
    key(&mut manager, KeyCode::Tab);
    key(&mut manager, KeyCode::Enter);
    assert!(manager.draft.as_ref().unwrap().file_roots.is_empty());
    assert_eq!(manager.draft.as_ref().unwrap().peers.len(), peers);
    assert_eq!(manager.page, Page::FileRoots);
    assert!(!manager.pending());
}

#[test]
fn directory_fields_accept_long_paths_but_keep_their_own_protocol_bounds() {
    let mut manager = manager();
    manager.page(Page::FileRoots);
    activate(&mut manager, Action::AddFileRoot);
    let path = format!(
        "/{}",
        vec![COMPONENT.repeat(COMPONENT_BYTES); DEEP_COMPONENTS].join("/")
    );
    assert!(path.len() > yolo_hub_federation_settings::MAX_SOCKET_PATH_BYTES);
    field(&mut manager, FieldId::RootPath, &path);
    assert_eq!(
        crate::wire_fields::raw_text(&manager.form.as_ref().unwrap().fields, FieldId::RootPath)
            .unwrap(),
        path
    );
    let form = manager.form.as_mut().unwrap();
    assert!(form.paste(&COMPONENT.repeat(yolo_hub_file_protocol::MAX_FILE_PATH_BYTES)));
    assert_eq!(
        crate::wire_fields::raw_text(&form.fields, FieldId::RootPath)
            .unwrap()
            .len(),
        yolo_hub_file_protocol::MAX_FILE_PATH_BYTES
    );
    let name = form
        .fields
        .iter_mut()
        .find(|field| field.id == FieldId::RootName)
        .unwrap();
    name.value = Value::Text {
        text: String::new(),
        cursor: Default::default(),
    };
    form.focused = form
        .fields
        .iter()
        .position(|field| field.id == FieldId::RootName)
        .unwrap();
    assert!(form.paste(&COMPONENT.repeat(yolo_hub_file_protocol::MAX_FILE_ROOT_NAME_BYTES)));
    assert!(!form.paste(COMPONENT));
}
