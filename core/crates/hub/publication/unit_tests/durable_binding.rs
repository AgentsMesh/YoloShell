use super::*;

const FOREIGN_DIR: &str = "foreign";
const RETIRED_SCHEMA: u16 = 0;

#[test]
fn foreign_directory_and_retired_record_schema_never_reset_publication_history() {
    for foreign_directory in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let state = HubState::open(&root.join(STATE_DIR), &root.join(HOST_DIR)).unwrap();
        let parent = state.directory().unwrap();
        let path = parent.path().join(PUBLICATION_DIR);
        let config = Configuration::default();
        let publication = Publication::open(
            parent.ensure_descendant(&path).unwrap(),
            state.peer_identity(),
            &config,
        )
        .unwrap();
        let mut record = publication.record.clone();
        if foreign_directory {
            record.directory = parent
                .ensure_descendant(&parent.path().join(FOREIGN_DIR))
                .unwrap()
                .durable_identity()
                .unwrap();
        } else {
            record.schema_version = RETIRED_SCHEMA;
        }
        publication
            .namespace
            .write_json(&publication.path, &record)
            .unwrap();
        let before = std::fs::read(&publication.path).unwrap();
        drop(publication);
        assert!(
            Publication::open(
                parent.open_descendant(&path).unwrap(),
                state.peer_identity(),
                &config,
            )
            .is_err()
        );
        assert_eq!(std::fs::read(path.join(RECORD_FILE)).unwrap(), before);
    }
}
