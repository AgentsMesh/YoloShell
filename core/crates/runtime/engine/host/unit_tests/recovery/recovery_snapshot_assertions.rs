use crate::host::tests::*;
pub(super) async fn assert_catalog_and_wire_snapshot(host: &Host, terminal_id: Uuid, expected: &[u8]) {
    let catalog = host.catalog_get(terminal_id).await.unwrap().unwrap();
    assert_manifest_snapshot(&catalog, expected);
    let response = host
        .dispatch(&request(
            method::LIST_SESSIONS,
            &ListSessionsRequest {
                include_archived: false,
                require_complete: false,
            },
        ))
        .await;
    let mut sessions = response
        .decode_payload::<ListSessionsResult>()
        .unwrap()
        .sessions
        .into_iter();
    assert_manifest_snapshot(&sessions.next().unwrap().manifest, expected);
    assert!(sessions.next().is_none());
}
