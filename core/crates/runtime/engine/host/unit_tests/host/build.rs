use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn host_build_is_separate_from_status_and_reports_the_loaded_image() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let status: HostStatus = host
        .dispatch(&request(method::HOST_STATUS, &()))
        .await
        .decode_payload()
        .unwrap();
    assert!(
        status
            .capabilities
            .iter()
            .any(|value| value == crate::protocol::capability::HOST_BUILD)
    );
    let expected = yolo_executable_identity::capture_current().unwrap();
    for _ in 0..BUILD_QUERY_REPETITIONS {
        let response = host.dispatch(&request(method::HOST_BUILD, &())).await;
        assert!(response.ok);
        assert_eq!(
            response
                .decode_payload::<yolo_build_image::BuildImage>()
                .unwrap(),
            expected
        );
    }
}

const BUILD_QUERY_REPETITIONS: usize = 2;
