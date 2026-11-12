use yolo_build_image::{BuildImage, SHA256_HEX_BYTES};
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{
    BUILD_METHODS, Command, Method, Reply, Request, Response, SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::PublicError;

const IMAGE_SIZE: u64 = 4096;
const DIGEST_CHARACTER: &str = "a";

#[test]
fn live_build_is_an_optional_nonmutating_local_method_with_typed_correlated_reply() {
    let command = Command::GetBuild {};
    assert_eq!(command.method(), Method::GetBuild);
    assert!(!command.mutates());
    assert!(command.local_only());
    assert!(!command.requires_read_reply_admission());
    assert_eq!(command.request_id(), None);
    assert!(!SUPPORTED_METHODS.contains(&Method::GetBuild));
    assert!(BUILD_METHODS.contains(&Method::GetBuild));
    let origin = HubId::new();
    let request = Request::new(origin, command).unwrap();
    let image = BuildImage::new(DIGEST_CHARACTER.repeat(SHA256_HEX_BYTES), IMAGE_SIZE).unwrap();
    let response = Response::new(
        &request,
        origin,
        Reply::Build {
            build: ServiceBuild {
                host: image.clone(),
                hub: image.clone(),
                file_helper: image,
            },
        },
    );
    response.validate_for(&request).unwrap();
    assert_eq!(
        serde_json::from_slice::<Response>(&serde_json::to_vec(&response).unwrap()).unwrap(),
        response
    );
    let hello = Request::new(origin, Command::Hello {}).unwrap();
    assert_eq!(
        Response::new(&hello, origin, response.reply).validate_for(&hello),
        Err(PublicError::InvalidPayload)
    );
    assert_eq!(
        Response::new(&request, origin, Reply::Completed {}).validate_for(&request),
        Err(PublicError::InvalidPayload)
    );
}
