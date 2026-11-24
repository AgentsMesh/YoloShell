use super::{constants::*, deadline, fixture::Fixture, output, shell};
use crate::hub::support::input;
use yolo_hub_client::{ClientError, HubEndpoint};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::AttachRequest;
use yolo_hub_terminal_client::Attachment;
use yolo_hub_jsonrpc_transport::TransportError;

#[tokio::test(flavor = "multi_thread")]
async fn live_client_revocation_retires_channels_without_killing_the_origin_shell() {
    let fixture = Fixture::start(&FULL_RIGHTS).await;
    let (session, original) = fixture.create().await;
    let target = session.fields().runtime.unwrap().target;
    let entry = fixture.entry();
    let mut client = fixture.connect(&entry).await;
    let mut output = Attachment::open(
        fixture.connect(&entry).await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    let spoofed =
        crate::hub::support::acquire(target, yolo_hub_terminal_protocol::LeaseMode::Acquire {});
    assert!(matches!(
        client.call(fixture.hub, spoofed, deadline()).await,
        Err(ClientError::Remote(
            yolo_hub_terminal_protocol::PublicError::Unauthorized
        ))
    ));
    fixture.no_writer(target).await;
    let grant = fixture.acquire(&mut client, target).await;
    let (command, marker) = shell::marker();
    client
        .call(fixture.hub, input(grant, &command), deadline())
        .await
        .unwrap();
    output::until(&mut output, &marker).await;
    // Independent origin observation stays live while the public client is revoked.
    let mut witness = Attachment::open(
        fixture.process.client().await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    fixture.configure(vec![]).await;
    output::closed(&mut output).await;
    let (forbidden, forbidden_marker) = shell::marker();
    output::assert_closed(
        client
            .call(fixture.hub, input(grant, &forbidden), deadline())
            .await
            .unwrap_err(),
    );
    let denied_entry = fixture.entry();
    let denied = HubEndpoint::remote(denied_entry.clone())
        .connect(fixture.hub, deadline())
        .await;
    let Err(ClientError::Transport(TransportError::Io(error))) = denied else {
        panic!("revoked key did not fail authentication")
    };
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    denied_entry.close(deadline()).await.unwrap();
    fixture.custody(&original).await;
    fixture.configure(vec![fixture.grant(&FULL_RIGHTS)]).await;
    // A restored grant never revives an already admitted stream or old client object.
    output::assert_closed(
        client
            .call(fixture.hub, input(grant, &forbidden), deadline())
            .await
            .unwrap_err(),
    );
    let fresh_entry = fixture.entry();
    let mut fresh = fixture.connect(&fresh_entry).await;
    let fresh_grant = fixture.acquire(&mut fresh, target).await;
    assert_ne!(
        fresh_grant.fields().principal.controller_instance_id,
        grant.fields().principal.controller_instance_id
    );
    let (command, marker) = shell::marker();
    fresh
        .call(fixture.hub, input(fresh_grant, &command), deadline())
        .await
        .unwrap();
    let observed = output::until(&mut witness, &marker).await;
    assert!(!String::from_utf8_lossy(&observed).contains(&forbidden_marker));
    fixture.custody(&original).await;
    let mut fresh_output = Attachment::open(
        fixture.connect(&fresh_entry).await,
        AttachRequest::new(target, None).unwrap(),
        deadline(),
    )
    .await
    .unwrap();
    fresh
        .call(
            fixture.hub,
            Command::Release { grant: fresh_grant },
            deadline(),
        )
        .await
        .unwrap();
    // Closing the outer OpenSSH transport retires all inner channels, never the PTY.
    fresh_entry.close(deadline()).await.unwrap();
    output::closed(&mut fresh_output).await;
    output::assert_closed(
        fresh
            .call(
                fixture.hub,
                Command::Get {
                    address: target.address,
                },
                deadline(),
            )
            .await
            .unwrap_err(),
    );
    fixture.no_writer(target).await;
    fixture.custody(&original).await;
    let mut local = fixture.process.client().await;
    let Reply::Lease { grant: local_grant } = local
        .call(
            fixture.hub,
            fixture.lease_command(target, yolo_hub_terminal_protocol::LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("local writer could not reacquire the surviving shell")
    };
    let (command, marker) = shell::marker();
    local
        .call(fixture.hub, input(local_grant, &command), deadline())
        .await
        .unwrap();
    output::until(&mut witness, &marker).await;
    local
        .call(
            fixture.hub,
            Command::Release { grant: local_grant },
            deadline(),
        )
        .await
        .unwrap();
    drop((local, witness, output, fresh_output, fresh, client));
    entry.close(deadline()).await.unwrap();
    fixture.stop().await;
}
