use crate::agent::claude::generation::constants::{AUTHORITY_REVISION_FIELD, REVISION_INCREMENT};
use serde_json::{Map, Value};
use uuid::Uuid;

pub(super) type PublicationMutation = fn(&mut Map<String, Value>);

pub(super) fn remove_publication_marker(transition: &mut Map<String, Value>) {
    transition.insert("predecessor_publication".to_owned(), Value::Null);
}

pub(super) fn add_publication_marker(transition: &mut Map<String, Value>) {
    let generation = transition
        .get("predecessor_publication_generation")
        .and_then(Value::as_u64)
        .unwrap();
    let predecessor = transition
        .get("predecessor_authority")
        .and_then(Value::as_object)
        .unwrap();
    let invocation_id = predecessor.get(stringify!(invocation_id)).cloned().unwrap();
    let revision = predecessor.get("revision").cloned().unwrap();
    transition.insert(
        "predecessor_publication".to_owned(),
        serde_json::json!({
            (stringify!(invocation_id)): invocation_id,
            "publication_generation": generation,
            (AUTHORITY_REVISION_FIELD): revision,
        }),
    );
}

pub(super) fn replace_publication_invocation(transition: &mut Map<String, Value>) {
    transition
        .get_mut("predecessor_publication")
        .and_then(Value::as_object_mut)
        .unwrap()
        .insert(
            stringify!(invocation_id).to_owned(),
            Uuid::new_v4().to_string().into(),
        );
}

pub(super) fn increment_predecessor_publication_generation(transition: &mut Map<String, Value>) {
    let generation = transition["predecessor_publication_generation"]
        .as_u64()
        .unwrap()
        .checked_add(REVISION_INCREMENT)
        .unwrap();
    transition.insert(
        "predecessor_publication_generation".to_owned(),
        generation.into(),
    );
}

pub(super) fn increment_publication_generation(transition: &mut Map<String, Value>) {
    let generation = transition
        .get("predecessor_publication_generation")
        .and_then(Value::as_u64)
        .unwrap()
        .checked_add(REVISION_INCREMENT)
        .unwrap();
    transition
        .get_mut("predecessor_publication")
        .and_then(Value::as_object_mut)
        .unwrap()
        .insert("publication_generation".to_owned(), generation.into());
}

pub(super) fn increment_publication_authority_revision(transition: &mut Map<String, Value>) {
    let revision = transition
        .get("predecessor_publication")
        .and_then(Value::as_object)
        .and_then(|publication| publication.get(AUTHORITY_REVISION_FIELD))
        .and_then(Value::as_u64)
        .unwrap()
        .checked_add(REVISION_INCREMENT)
        .unwrap();
    transition
        .get_mut("predecessor_publication")
        .and_then(Value::as_object_mut)
        .unwrap()
        .insert(AUTHORITY_REVISION_FIELD.to_owned(), revision.into());
}
