use super::{
    constants::*,
    fixtures::{directory, open},
    view::view,
};
use yolo_hub_agent_projection::AgentProjection;
use yolo_hub_identity::{AgentAddress, AgentId, HubId};
use yolo_paths::{directory::PrivateDirectory, namespace_lease::NamespaceLease};

#[test]
fn invalid_host_binding_never_mutates_persisted_projection() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let source = view();
    let first = store.project(&source).unwrap().unwrap();
    let baseline = store.list().unwrap();
    let mutations: &[fn(&mut yolo_agent_contract::AgentBinding)] = &[
        |b| b.schema_version += 1,
        |b| b.terminal_id = uuid::Uuid::new_v4(),
        |b| b.updated_at_ms = b.detected_at_ms - 1,
        |b| b.process_pid = Some(0),
        |b| b.process_start_time = Some(0),
        |b| b.data_home = String::from("relative"),
        |b| b.provider_session_id.clear(),
    ];
    for mutate in mutations {
        let mut changed = source.clone();
        mutate(changed.agent.as_mut().unwrap());
        assert!(store.project(&changed).is_err());
        assert_eq!(store.list().unwrap(), baseline);
    }
    assert!(
        store
            .get(AgentAddress {
                hub_id: HubId::new(),
                ..first.cursor().target().address
            })
            .is_err()
    );
    assert!(
        store
            .get(AgentAddress {
                hub_id: hub,
                agent_id: AgentId::new()
            })
            .unwrap()
            .is_none()
    );
}

#[test]
fn retained_namespace_and_single_writer_survive_neither_replacement_nor_clone() {
    let (root, path, hub) = directory();
    let mut store = open(&path, hub);
    store.project(&view()).unwrap().unwrap();
    assert!(AgentProjection::open(PrivateDirectory::open(&path).unwrap(), hub).is_err());
    let clone = root.path().canonicalize().unwrap().join(CLONED_DIRECTORY);
    let directory = PrivateDirectory::ensure(&clone).unwrap();
    let namespace = NamespaceLease::capture(&[(RETAINED_LABEL, &path)]).unwrap();
    let record: serde_json::Value = namespace.read_json(&path.join(STATE_FILE)).unwrap();
    let clone_namespace =
        NamespaceLease::from_directories(vec![(RETAINED_LABEL, directory)]).unwrap();
    clone_namespace
        .write_json(&clone.join(STATE_FILE), &record)
        .unwrap();
    assert!(AgentProjection::open(PrivateDirectory::open(&clone).unwrap(), hub).is_err());
    let moved = root.path().canonicalize().unwrap().join(MOVED_DIRECTORY);
    std::fs::rename(&path, &moved).unwrap();
    PrivateDirectory::ensure(&path).unwrap();
    assert!(store.verify().is_err());
    assert!(store.list().is_err());
    assert!(store.mark_unreachable().is_err());
    assert!(store.project(&view()).is_err());
}

#[test]
fn corrupt_state_is_not_replaced_with_new_agent_identities() {
    let (_root, path, hub) = directory();
    let mut store = open(&path, hub);
    let first = store.project(&view()).unwrap().unwrap();
    drop(store);
    let namespace = NamespaceLease::capture(&[(RETAINED_LABEL, &path)]).unwrap();
    let mut record: serde_json::Value = namespace.read_json(&path.join(STATE_FILE)).unwrap();
    record["schema"] = u16::MAX.into();
    namespace
        .write_json(&path.join(STATE_FILE), &record)
        .unwrap();
    assert!(AgentProjection::open(PrivateDirectory::open(&path).unwrap(), hub).is_err());
    let retained: serde_json::Value = namespace.read_json(&path.join(STATE_FILE)).unwrap();
    assert!(
        retained["entries"]
            .as_object()
            .unwrap()
            .contains_key(&first.cursor().target().address.agent_id.to_string())
    );
    assert_eq!(retained, record);
}
