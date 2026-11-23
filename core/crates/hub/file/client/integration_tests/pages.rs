use crate::fixture::*;
use yolo_hub_file_client::{FileClientError, list_roots};
use yolo_hub_file_protocol::{FileRootSummary, MAX_FILE_ROOT_PAGE, MAX_FILE_ROOTS};
use yolo_hub_identity::FileRootEpoch;
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn pages_are_collected_for_the_requested_origin_without_exposing_partial_results() {
    let (origin, epoch, all) = roots(MAX_FILE_ROOT_PAGE + 1);
    let cursor = all[MAX_FILE_ROOT_PAGE - 1].fields().target.address.root_id;
    let (mut client, server) = connected(vec![
        Reply::FileRoots {
            epoch,
            roots: all[..MAX_FILE_ROOT_PAGE].to_vec(),
            next: Some(cursor),
        },
        Reply::FileRoots {
            epoch,
            roots: all[MAX_FILE_ROOT_PAGE..].to_vec(),
            next: None,
        },
    ])
    .await;
    let result = list_roots(&mut client, origin, deadline()).await.unwrap();
    assert_eq!(result.origin, origin);
    assert_eq!(result.epoch, epoch);
    assert_eq!(result.roots, all);
    assert_eq!(
        server.await.unwrap(),
        vec![
            Command::ListFileRoots { after: None },
            Command::ListFileRoots {
                after: Some(cursor)
            }
        ]
    );
}

#[tokio::test]
async fn a_new_scope_generation_between_pages_is_not_merged_or_retried() {
    let (origin, epoch, all) = roots(MAX_FILE_ROOT_PAGE + 1);
    let next_epoch = FileRootEpoch::new();
    let mut changed = all[MAX_FILE_ROOT_PAGE].fields().clone();
    changed.target.epoch = next_epoch;
    let (mut client, server) = connected(vec![
        Reply::FileRoots {
            epoch,
            roots: all[..MAX_FILE_ROOT_PAGE].to_vec(),
            next: Some(all[MAX_FILE_ROOT_PAGE - 1].fields().target.address.root_id),
        },
        Reply::FileRoots {
            epoch: next_epoch,
            roots: vec![FileRootSummary::new(changed).unwrap()],
            next: None,
        },
    ])
    .await;
    assert!(matches!(
        list_roots(&mut client, origin, deadline()).await,
        Err(FileClientError::Changed)
    ));
    assert_eq!(server.await.unwrap().len(), 2);
}

#[tokio::test]
async fn empty_lists_have_an_epoch_and_origins_cannot_force_unbounded_pagination() {
    let (origin, epoch, _) = roots(0);
    let (mut client, server) = connected(vec![Reply::FileRoots {
        epoch,
        roots: vec![],
        next: None,
    }])
    .await;
    assert!(
        list_roots(&mut client, origin, deadline())
            .await
            .unwrap()
            .roots
            .is_empty()
    );
    server.await.unwrap();
    let (origin, epoch, all) = roots(MAX_FILE_ROOTS);
    let replies = all
        .chunks(MAX_FILE_ROOT_PAGE)
        .map(|roots| Reply::FileRoots {
            epoch,
            roots: roots.to_vec(),
            next: Some(roots.last().unwrap().fields().target.address.root_id),
        })
        .collect();
    let (mut client, server) = connected(replies).await;
    assert!(matches!(
        list_roots(&mut client, origin, deadline()).await,
        Err(FileClientError::Limit)
    ));
    assert_eq!(
        server.await.unwrap().len(),
        MAX_FILE_ROOTS / MAX_FILE_ROOT_PAGE
    );
}
