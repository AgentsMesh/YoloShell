use super::*;
use yolo_build_image::{BuildImage, SHA256_HEX_BYTES};
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_dispatch::{BuildFuture, BuildProvider};
use yolo_hub_rpc_protocol::Method;

const IMAGE_SIZE: u64 = 4096;
const DIGEST_CHARACTER: &str = "a";

struct Provider(Result<ServiceBuild, PublicError>);
impl BuildProvider for Provider {
    fn build(&self) -> BuildFuture<'_> {
        Box::pin(async { self.0.clone() })
    }
}
fn image() -> ServiceBuild {
    let image = BuildImage::new(DIGEST_CHARACTER.repeat(SHA256_HEX_BYTES), IMAGE_SIZE).unwrap();
    ServiceBuild {
        host: image.clone(),
        hub: image.clone(),
        file_helper: image,
    }
}

#[tokio::test]
async fn embedded_hub_has_no_fabricated_build_or_advertised_capability() {
    let (_directory, hub, receiver) = fixture();
    assert!(matches!(
        hub.dispatch(command(&hub, Command::GetBuild {})).await,
        Err(PublicError::Unsupported)
    ));
    let Operation::Reply(Reply::Hello { hello }) = hub
        .dispatch(command(&hub, Command::Hello {}))
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(!hello.methods.contains(&Method::GetBuild));
    assert!(receiver.is_empty());
}

#[tokio::test]
async fn installed_provider_is_local_only_and_failure_never_mutates_configuration() {
    for outcome in [Ok(image()), Err(PublicError::Internal)] {
        let (_directory, mut hub, receiver) = fixture();
        Arc::get_mut(&mut hub).unwrap().build = Some(Arc::new(Provider(outcome.clone())));
        let request = command(&hub, Command::GetBuild {});
        let result = hub.dispatch(request.clone()).await;
        match outcome {
            Ok(expected) => {
                let Operation::Reply(reply) = result.unwrap() else {
                    panic!()
                };
                Response::new(&request, hub.hub_id(), reply.clone())
                    .validate_for(&request)
                    .unwrap();
                assert_eq!(reply, Reply::Build { build: expected });
            }
            Err(expected) => assert!(matches!(result, Err(actual) if actual == expected)),
        }
        let Operation::Reply(Reply::Hello { hello }) = hub
            .dispatch(command(&hub, Command::Hello {}))
            .await
            .unwrap()
        else {
            panic!()
        };
        assert!(hello.methods.contains(&Method::GetBuild));
        let request = Request::new(HubId::new(), Command::GetBuild {}).unwrap();
        assert!(matches!(
            hub.dispatch(request).await,
            Err(PublicError::Unauthorized)
        ));
        assert!(receiver.is_empty());
    }
}
