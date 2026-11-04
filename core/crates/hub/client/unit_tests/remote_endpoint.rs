use crate::{ClientError, EntryConnector, HubEndpoint, HubEntryStream};
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::time::Instant;
use uuid::Uuid;
use yolo_hub_identity::HubId;

#[derive(Debug)]
struct Connector {
    client: Uuid,
    fail_validation: bool,
}
impl EntryConnector for Connector {
    fn validate(&self) -> Result<(), ClientError> {
        if self.fail_validation {
            Err(ClientError::InvalidEndpoint)
        } else {
            Ok(())
        }
    }
    fn client_id(&self) -> Uuid {
        self.client
    }
    fn open(
        &self,
        _: HubId,
        _: Instant,
    ) -> Pin<Box<dyn Future<Output = Result<HubEntryStream, ClientError>> + Send + '_>> {
        Box::pin(async { Err(ClientError::WrongLocalUser) })
    }
}

#[tokio::test]
async fn injected_entry_has_independent_identity_and_never_falls_back_to_local_socket() {
    let client = Uuid::new_v4();
    let endpoint = HubEndpoint::remote(Arc::new(Connector {
        client,
        fail_validation: false,
    }));
    assert_eq!(endpoint.client_id(), Some(client));
    assert_eq!(endpoint.local_socket(), None);
    assert!(endpoint.validate().is_ok());
    assert_eq!(endpoint, endpoint.clone());
    assert_ne!(
        endpoint,
        HubEndpoint::remote(Arc::new(Connector {
            client,
            fail_validation: false
        }))
    );
    let local = HubEndpoint::local(crate::unit_tests::fixtures::constants::SOCKET_NAME);
    assert_ne!(endpoint, local);
    assert_ne!(local, endpoint);
    assert_eq!(local.client_id(), None);
    assert!(format!("{endpoint:?}").contains(&client.to_string()));
    assert!(matches!(
        endpoint.connect(HubId::new(), Instant::now()).await,
        Err(ClientError::WrongLocalUser)
    ));
}

#[test]
fn adapter_validation_and_non_nil_identity_are_required_before_opening() {
    let nil = HubEndpoint::remote(Arc::new(Connector {
        client: Uuid::nil(),
        fail_validation: false,
    }));
    assert!(matches!(nil.validate(), Err(ClientError::InvalidEndpoint)));
    let invalid = HubEndpoint::remote(Arc::new(Connector {
        client: Uuid::new_v4(),
        fail_validation: true,
    }));
    assert!(matches!(
        invalid.validate(),
        Err(ClientError::InvalidEndpoint)
    ));
}
