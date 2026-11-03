use crate::fixture::*;
use yolo_hub_file_protocol::*;
use yolo_hub_file_reader_test_support::*;
use yolo_hub_file_service::FileService;
use yolo_hub_identity::{FileRootId, HubId};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn roots_are_preflighted_then_published_with_one_epoch_and_exact_read_targets() {
    let fixture = Fixture::new();
    let configured = vec![configuration(&fixture)];
    let service = service(&fixture, &configured).await.unwrap();
    assert!(service.configured_as(&configured));
    assert!(!service.configured_as(&[]));
    let (epoch, roots, next) = service.list(None).unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0].fields().target.epoch, epoch);
    assert!(next.is_none());
    let read = request(&roots[0]);
    assert_eq!(
        service.read(read.clone()).await.unwrap().fields().data,
        PAYLOAD[..READ_LENGTH as usize]
    );
    let mut wrong = read.fields().clone();
    wrong.target.address.hub_id = HubId::new();
    assert_eq!(
        service.read(ReadFileRequest::new(wrong).unwrap()).await,
        Err(PublicError::OriginMismatch)
    );
    let mut wrong = read.fields().clone();
    wrong.target.address.root_id = FileRootId::new();
    assert_eq!(
        service.read(ReadFileRequest::new(wrong).unwrap()).await,
        Err(PublicError::NotFound)
    );
    assert_eq!(
        service.read(fixture.request(FILE_PATH)).await,
        Err(PublicError::TargetMismatch)
    );
    service.close();
    assert!(!service.configured_as(&configured));
    assert_eq!(service.list(None), Err(PublicError::Unreachable));
    assert_eq!(service.read(read).await, Err(PublicError::Unreachable));
}

#[tokio::test]
async fn pages_are_sorted_bounded_and_share_the_same_preflight_generation() {
    let fixture = Fixture::new();
    let configured = (0..=MAX_FILE_ROOT_PAGE)
        .map(|_| {
            let mut fields = configuration(&fixture).fields().clone();
            fields.root_id = FileRootId::new();
            FileRootConfiguration::new(fields).unwrap()
        })
        .collect::<Vec<_>>();
    let service = service(&fixture, &configured).await.unwrap();
    let (epoch, first, next) = service.list(None).unwrap();
    assert_eq!(first.len(), MAX_FILE_ROOT_PAGE);
    assert!(
        first
            .windows(2)
            .all(|pair| pair[0].fields().target.address.root_id
                < pair[1].fields().target.address.root_id)
    );
    let next = next.unwrap();
    assert_eq!(first.last().unwrap().fields().target.address.root_id, next);
    let (second_epoch, second, next) = service.list(Some(next)).unwrap();
    assert_eq!(second_epoch, epoch);
    assert_eq!(second.len(), 1);
    assert!(next.is_none());
    assert_eq!(
        service
            .list(Some(second[0].fields().target.address.root_id))
            .unwrap()
            .1
            .len(),
        0
    );
}

#[tokio::test]
async fn invalid_preflight_does_not_create_a_service_or_weaken_private_exclusions() {
    let fixture = Fixture::new();
    let original = configuration(&fixture);
    assert!(matches!(
        service(&fixture, &[original.clone(), original.clone()]).await,
        Err(PublicError::InvalidRequest)
    ));
    assert!(matches!(
        service(&fixture, &vec![original.clone(); MAX_FILE_ROOTS + 1]).await,
        Err(PublicError::InvalidRequest)
    ));
    let mut fields = original.fields().clone();
    fields.path = fixture.private.to_str().unwrap().into();
    assert!(matches!(
        service(&fixture, &[FileRootConfiguration::new(fields).unwrap()]).await,
        Err(PublicError::Unauthorized)
    ));
    assert!(matches!(
        FileService::prepare(
            fixture.summary.fields().target.address.hub_id,
            &[original],
            vec![fixture.private.clone()],
            None
        )
        .await,
        Err(PublicError::Unsupported)
    ));
}

#[tokio::test]
async fn empty_configuration_needs_no_helper_and_cannot_read_arbitrary_paths() {
    let fixture = Fixture::new();
    let service = FileService::prepare(
        fixture.summary.fields().target.address.hub_id,
        &[],
        vec![],
        None,
    )
    .await
    .unwrap();
    let (_, roots, next) = service.list(None).unwrap();
    assert!(roots.is_empty() && next.is_none());
    assert_eq!(
        service.read(fixture.request(FILE_PATH)).await,
        Err(PublicError::NotFound)
    );
}

#[tokio::test]
async fn scope_replacement_and_directory_replacement_are_distinct_fences() {
    let fixture = Fixture::new();
    let configured = vec![configuration(&fixture)];
    let first = service(&fixture, &configured).await.unwrap();
    let second = service(&fixture, &configured).await.unwrap();
    let root = first.list(None).unwrap().1.remove(0);
    let request = request(&root);
    assert_eq!(
        second.read(request.clone()).await,
        Err(PublicError::TargetMismatch)
    );
    std::fs::rename(&fixture.workspace, fixture.base.join(SAVED_NAME)).unwrap();
    std::fs::create_dir_all(fixture.workspace.join(SOURCE_DIRECTORY)).unwrap();
    std::fs::write(fixture.workspace.join(FILE_PATH), PAYLOAD).unwrap();
    assert_eq!(first.read(request).await, Err(PublicError::Conflict));
}
