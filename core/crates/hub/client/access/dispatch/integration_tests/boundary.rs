use super::{backend::*, model::*};
use yolo_hub_client_access_dispatch::ScopedDispatcher;
use yolo_hub_dispatch::Dispatcher;
use yolo_hub_routing::Right;
use yolo_hub_rpc_protocol::{
    BUILD_METHODS, Command, HUB_PROTOCOL_VERSION, Hello, MANAGEMENT_METHODS, Method, Reply,
    SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::PublicError;

#[tokio::test]
async fn hello_advertises_only_authorized_resource_operations() {
    let (_registry, admission) = authority(&[Right::Observe]);
    let methods = [SUPPORTED_METHODS, MANAGEMENT_METHODS, BUILD_METHODS].concat();
    let backend = Scripted::new([Reply::Hello {
        hello: Hello {
            hub_id: hub(ENTRY),
            version: HUB_PROTOCOL_VERSION,
            methods,
        },
    }]);
    let scoped = ScopedDispatcher::new(backend, admission).unwrap();
    let Reply::Hello { hello } = reply(scoped.dispatch(request(ENTRY, Command::Hello {})).await)
    else {
        panic!()
    };
    assert!(hello.methods.contains(&Method::Hello));
    assert!(hello.methods.contains(&Method::ListAgents));
    for denied in [
        Method::Configure,
        Method::Create,
        Method::Lease,
        Method::GetBuild,
        Method::ReadFile,
        Method::AgentControl,
    ] {
        assert!(!hello.methods.contains(&denied));
    }
}

#[tokio::test]
async fn unauthorized_origin_and_spoofed_writer_never_reach_the_backend() {
    let (_registry, admission) = authority(&[Right::Observe, Right::TerminalWrite]);
    let backend = Scripted::new([]);
    let scoped = ScopedDispatcher::new(backend.clone(), admission).unwrap();
    let mut denied = writer_commands(OTHER_CLIENT);
    denied.push(Command::GetBuild {});
    for command in denied {
        assert!(matches!(
            scoped.dispatch(request(ORIGIN, command)).await,
            Err(PublicError::Unauthorized)
        ));
    }
    assert!(matches!(
        scoped
            .dispatch(request(
                HIDDEN,
                Command::Get {
                    address: address(HIDDEN, SESSION)
                }
            ))
            .await,
        Err(PublicError::Unauthorized)
    ));
    assert!(backend.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn backend_reply_cannot_substitute_an_unrelated_origin() {
    let (_registry, admission) = authority(&[Right::Observe]);
    let scoped = ScopedDispatcher::new(
        Scripted::new([Reply::Terminal {
            terminal: summary(HIDDEN, SESSION),
        }]),
        admission,
    )
    .unwrap();
    assert!(matches!(
        scoped
            .dispatch(request(
                ORIGIN,
                Command::Get {
                    address: address(ORIGIN, SESSION)
                }
            ))
            .await,
        Err(PublicError::InvalidPayload)
    ));
}

#[test]
fn an_admission_cannot_be_transferred_to_a_different_entry() {
    let (_registry, admission) = authority(&[Right::Observe]);
    let mut backend = Scripted::new([]);
    std::sync::Arc::get_mut(&mut backend).unwrap().origin = hub(HIDDEN);
    assert!(matches!(
        ScopedDispatcher::new(backend, admission),
        Err(PublicError::OriginMismatch)
    ));
}
