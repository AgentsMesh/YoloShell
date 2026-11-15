use super::{constants::*, fixture::*, model::*};
use std::fs;
use yolo_hub_client_access_runtime::ClientAccessRuntime;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;
use yolo_paths::directory::PrivateDirectory;

#[tokio::test]
async fn identical_record_replacement_closes_authority_and_reports_uncertain_configure() {
    let fixture = Fixture::new();
    let original = fixture.current().await;
    let registry = fixture.runtime.registry();
    let admission = registry
        .admit_verified_key(&client_key(), u64::MIN)
        .unwrap();
    let record = fixture.record();
    let bytes = fs::read(&record).unwrap();
    fs::rename(&record, fixture.sibling(RETIRED_RECORD)).unwrap();
    write_new(&record, &bytes);
    assert!(matches!(
        fixture
            .call(Command::ConfigureClientAccess {
                change: change(&original)
            })
            .await,
        Err(PublicError::OutcomeUnknown)
    ));
    assert!(!admission.is_current(u64::MIN));
    assert!(registry.current().is_err());
    assert_eq!(fs::read(&record).unwrap(), bytes);
    assert!(matches!(
        fixture.call(Command::GetClientAccess {}).await,
        Err(PublicError::Unreachable)
    ));
    let state = fixture.state.clone();
    drop(fixture.runtime);
    let restored = ClientAccessRuntime::open(state, fixture.backend.clone(), None).unwrap();
    assert_eq!(
        call(restored.as_ref(), Command::GetClientAccess {})
            .await
            .unwrap(),
        Reply::ClientAccess {
            configuration: original
        }
    );
}

#[tokio::test]
async fn namespace_replacement_causes_read_failure_and_restoring_the_path_cannot_restore_authority()
{
    let fixture = Fixture::new();
    let original = fixture.current().await;
    let admission = fixture
        .runtime
        .registry()
        .admit_verified_key(&client_key(), u64::MIN)
        .unwrap();
    let directory = fixture.access_directory();
    let retired = fixture.sibling(RETIRED_DIRECTORY);
    fs::rename(&directory, &retired).unwrap();
    PrivateDirectory::open_or_create_strict(&directory).unwrap();
    let before = fixture.backend.calls();
    assert!(matches!(
        fixture.call(Command::Hello {}).await,
        Err(PublicError::Unreachable)
    ));
    assert!(matches!(
        fixture
            .call(Command::List {
                include_archived: false,
                after: None
            })
            .await,
        Err(PublicError::Unreachable)
    ));
    assert_eq!(fixture.backend.calls(), before);
    assert!(!admission.is_current(u64::MIN));
    assert!(!directory.join(RECORD_FILE).exists());
    fs::rename(&directory, fixture.sibling(REPLACEMENT_DIRECTORY)).unwrap();
    fs::rename(retired, &directory).unwrap();
    assert!(matches!(
        fixture
            .call(Command::ConfigureClientAccess {
                change: change(&original)
            })
            .await,
        Err(PublicError::OutcomeUnknown)
    ));
}

#[tokio::test]
async fn in_memory_registry_changes_cannot_replace_the_persisted_authority() {
    let fixture = Fixture::new();
    let original = fixture.current().await;
    let registry = fixture.runtime.registry();
    let bytes = fs::read(fixture.record()).unwrap();
    let altered = change(&original).configuration;
    registry.replace(original.revision, altered).unwrap();
    let admission = registry
        .admit_verified_key(&client_key(), u64::MIN)
        .unwrap();
    assert!(matches!(
        fixture.call(Command::GetClientAccess {}).await,
        Err(PublicError::Unreachable)
    ));
    assert!(!admission.is_current(u64::MIN));
    assert!(registry.current().is_err());
    assert_eq!(fs::read(fixture.record()).unwrap(), bytes);
}
