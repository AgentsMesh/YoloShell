use crate::{
    CONFIG_SCHEMA_VERSION, MAX_READY_BYTES, ReadyDocument, encode_ready, readiness::read_ready,
};
use yolo_hub_identity::HubId;

#[tokio::test]
async fn readiness_is_a_bounded_framed_capnp_document_with_clean_eof_distinct_from_truncation() {
    let expected = ReadyDocument {
        schema_version: CONFIG_SCHEMA_VERSION,
        entry_hub: HubId::new(),
        socket_path: "/tmp/yolo-ready.sock".into(),
    };
    let encoded = encode_ready(&expected).unwrap();
    assert_eq!(
        read_ready(&mut encoded.as_slice()).await.unwrap(),
        Some(expected)
    );
    assert_eq!(read_ready(&mut &b""[..]).await.unwrap(), None);
    for end in 1..encoded.len() {
        assert!(read_ready(&mut &encoded[..end]).await.is_err());
    }
    assert!(
        read_ready(
            &mut u32::try_from(MAX_READY_BYTES + 1)
                .unwrap()
                .to_be_bytes()
                .as_slice()
        )
        .await
        .is_err()
    );
    assert!(read_ready(&mut &b"{}\n"[..]).await.is_err());
}
