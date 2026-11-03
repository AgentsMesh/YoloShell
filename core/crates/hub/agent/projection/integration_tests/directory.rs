use super::{
    constants::*,
    fixtures::{directory, open},
    view::view,
};
use yolo_hub_agent_protocol::{
    AGENT_WORKING_DIRECTORY_FIELD, Freshness, MAX_AGENT_WORKING_DIRECTORY_BYTES, Reachability,
};
use yolo_paths::namespace_lease::NamespaceLease;

#[test]
fn observed_agent_directory_changes_without_replacing_its_execution_or_shell_directory() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let mut source = view();
    source.agent.as_mut().unwrap().cwd = AGENT_DIRECTORY.into();
    let first = store.project(&source).unwrap().unwrap();
    assert_eq!(
        first.fields().working_directory.as_ref().unwrap().as_str(),
        AGENT_DIRECTORY
    );
    assert_eq!(source.manifest.cwd.to_str(), Some(WORKING_DIRECTORY));
    let binding = source.agent.as_mut().unwrap();
    binding.cwd = CHANGED_AGENT_DIRECTORY.into();
    binding.updated_at_ms += IDENTITY_STEP as i64;
    let changed = store.project(&source).unwrap().unwrap();
    assert_eq!(changed.cursor().target(), first.cursor().target());
    assert_eq!(changed.fields().terminal, first.fields().terminal);
    assert_eq!(changed.cursor().stream(), first.cursor().stream());
    assert!(changed.cursor().sequence() > first.cursor().sequence());
    assert_eq!(
        changed
            .fields()
            .working_directory
            .as_ref()
            .unwrap()
            .as_str(),
        CHANGED_AGENT_DIRECTORY
    );
    assert_eq!(store.project(&source).unwrap().unwrap(), changed);
    store.mark_unreachable().unwrap();
    let stale = store.get(first.cursor().target().address).unwrap().unwrap();
    assert_eq!(
        stale.fields().working_directory,
        changed.fields().working_directory
    );
    assert_eq!(stale.fields().freshness, Freshness::Stale);
    assert_eq!(stale.fields().reachability, Reachability::Unreachable);
    drop(store);
    let mut reopened = open(&path, hub);
    let restored = reopened.project(&source).unwrap().unwrap();
    assert_eq!(restored.cursor().target(), first.cursor().target());
    assert_eq!(
        restored.fields().working_directory,
        changed.fields().working_directory
    );
    assert_eq!(restored.fields().freshness, Freshness::Current);
}

#[test]
fn invalid_directory_cannot_overwrite_valid_observation_or_persisted_identity() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let source = view();
    let first = store.project(&source).unwrap().unwrap();
    let baseline = std::fs::read(path.join(STATE_FILE)).unwrap();
    let oversized = format!("/{AGENT_DIRECTORY}").repeat(MAX_AGENT_WORKING_DIRECTORY_BYTES);
    for invalid in ["relative", "/bad\npath", "/bad\0path", oversized.as_str()] {
        let mut changed = source.clone();
        changed.agent.as_mut().unwrap().cwd = invalid.into();
        assert!(store.project(&changed).is_err());
        assert_eq!(
            store.get(first.cursor().target().address).unwrap(),
            Some(first.clone())
        );
        assert_eq!(std::fs::read(path.join(STATE_FILE)).unwrap(), baseline);
    }
}

#[test]
fn retained_record_without_directory_reopens_as_unknown_then_reconfirms_same_execution() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let source = view();
    let first = store.project(&source).unwrap().unwrap();
    drop(store);
    let namespace = NamespaceLease::capture(&[(RETAINED_LABEL, &path)]).unwrap();
    let mut record: serde_json::Value = namespace.read_json(&path.join(STATE_FILE)).unwrap();
    let key = first.cursor().target().address.agent_id.to_string();
    record[ENTRIES_FIELD][&key][SNAPSHOT_FIELD]
        .as_object_mut()
        .unwrap()
        .remove(AGENT_WORKING_DIRECTORY_FIELD)
        .unwrap();
    namespace
        .write_json(&path.join(STATE_FILE), &record)
        .unwrap();
    let mut reopened = open(&path, hub);
    let unknown = reopened
        .get(first.cursor().target().address)
        .unwrap()
        .unwrap();
    assert_eq!(unknown.cursor().target(), first.cursor().target());
    assert_eq!(unknown.fields().working_directory, None);
    assert_eq!(unknown.fields().freshness, Freshness::Stale);
    let current = reopened.project(&source).unwrap().unwrap();
    assert_eq!(current.cursor().target(), first.cursor().target());
    assert_eq!(
        current.fields().working_directory,
        first.fields().working_directory
    );
    assert!(current.cursor().sequence() > unknown.cursor().sequence());
}
