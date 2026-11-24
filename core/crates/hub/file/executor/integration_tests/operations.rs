use crate::{constants::*, fixture::*};
use std::fs;
use yolo_hub_file_executor::FileExecutor;
use yolo_hub_file_helper_protocol::Payload;
use yolo_hub_file_reader_test_support::*;
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn real_helper_inspects_and_reads_with_the_exact_approved_directory_fingerprint() {
    let fixture = Fixture::new();
    let executor = executor();
    let Payload::Inspected { bindings } = executor
        .execute(inspect(&fixture), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0].root, fixture.summary);
    assert_eq!(
        bindings[0].fingerprint,
        fixture.root().fingerprint().unwrap()
    );
    let Payload::Read { file } = executor
        .execute(read(&fixture, bindings[0].clone()), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(file.fields().data, PAYLOAD[..READ_LENGTH as usize]);
    assert!(!file.fields().eof);
    executor.close();
}

#[tokio::test]
async fn a_new_helper_cannot_adopt_a_directory_replaced_since_approval() {
    let fixture = Fixture::new();
    let executor = executor();
    let Payload::Inspected { bindings } = executor
        .execute(inspect(&fixture), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    fs::rename(&fixture.workspace, fixture.base.join(SAVED_NAME)).unwrap();
    fs::create_dir_all(fixture.workspace.join(SOURCE_DIRECTORY)).unwrap();
    fs::write(fixture.workspace.join(FILE_PATH), PAYLOAD).unwrap();
    assert_eq!(
        executor
            .execute(read(&fixture, bindings[0].clone()), deadline())
            .await
            .unwrap(),
        Payload::Error {
            code: PublicError::Conflict
        }
    );
}

#[tokio::test]
async fn registration_cannot_export_the_protected_namespace() {
    let fixture = Fixture::new();
    let mut fields = inspect(&fixture).fields().clone();
    fields.protected = vec![fixture.base.to_str().unwrap().into()];
    let request = yolo_hub_file_helper_protocol::Request::new(fields).unwrap();
    assert_eq!(
        executor().execute(request, deadline()).await.unwrap(),
        Payload::Error {
            code: PublicError::Unauthorized
        }
    );
}

#[tokio::test]
async fn closed_and_expired_executors_cannot_start_more_work() {
    let fixture = Fixture::new();
    let executor = executor();
    assert_eq!(
        executor
            .execute(inspect(&fixture), tokio::time::Instant::now())
            .await,
        Err(PublicError::Unreachable)
    );
    executor.clone().close();
    assert_eq!(
        executor.execute(inspect(&fixture), deadline()).await,
        Err(PublicError::Unreachable)
    );
    executor.close();
    assert!(FileExecutor::new(UNKNOWN_HELPER_NAME.into()).is_err());
    assert!(FileExecutor::new(std::path::PathBuf::from(std::path::MAIN_SEPARATOR_STR)).is_err());
    let absent = FileExecutor::new(fixture.base.join(UNKNOWN_HELPER_NAME)).unwrap();
    assert_eq!(
        absent.execute(inspect(&fixture), deadline()).await,
        Err(PublicError::Unreachable)
    );
}
