use super::{
    files_support::{self, constants::*},
    fixture::*,
};
use crate::{
    constants::INVALID,
    fields::{FieldId, Value},
    form::Form,
    model::Action,
};
use yolo_hub_file_protocol::FileRootConfiguration;
use yolo_hub_identity::FileRootId;

#[test]
fn stale_editors_cannot_restore_removed_roots_or_replace_newer_root_metadata() {
    for removed in [false, true] {
        let mut manager = manager();
        let root = files_support::add(&mut manager);
        activate(&mut manager, Action::EditFileRoot(root.fields().root_id));
        if removed {
            manager.draft.as_mut().unwrap().file_roots.clear();
        } else {
            let mut fields = root.fields().clone();
            fields.path = CHANGED_PATH.into();
            manager.draft.as_mut().unwrap().file_roots[0] =
                FileRootConfiguration::new(fields).unwrap();
        }
        let expected = manager.draft.clone();
        save(&mut manager);
        assert_eq!(manager.status, INVALID);
        assert_eq!(manager.draft, expected);
        manager.remove(Action::RemoveFileRoot(root));
        assert_eq!(manager.draft, expected);
    }
}

#[test]
fn read_only_identity_corruption_and_duplicate_creation_are_rejected_atomically() {
    let mut manager = manager();
    let root = files_support::add(&mut manager);
    let current = manager.draft.as_ref().unwrap();
    let local = manager.base.as_ref().unwrap().state.identity;
    let mut form = Form::file_root(Some(&root));
    form.fields
        .iter_mut()
        .find(|field| field.id == FieldId::RootId)
        .unwrap()
        .value = Value::Text {
        text: FileRootId::new().to_string(),
        cursor: Default::default(),
    };
    assert!(form.commit(current, local).is_err());
    let mut fresh = Form::file_root(None);
    fresh
        .fields
        .iter_mut()
        .find(|field| field.id == FieldId::RootName)
        .unwrap()
        .value = Value::Text {
        text: NAME.into(),
        cursor: Default::default(),
    };
    fresh
        .fields
        .iter_mut()
        .find(|field| field.id == FieldId::RootPath)
        .unwrap()
        .value = Value::Text {
        text: PATH.into(),
        cursor: Default::default(),
    };
    let added = fresh.commit(current, local).unwrap();
    assert!(fresh.commit(&added, local).is_err());
}
