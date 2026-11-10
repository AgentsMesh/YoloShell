use super::{
    assets::Assets,
    constants::{OTHER_NAME, SECOND_DELIVERY, TARGET_NAME},
    observe,
};
use crate::hub::federation_support::{RunningNode, deadline};
use crate::{Harness, runtime_ref_from_view};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};
#[path = "../federation/federation_tui_capture.rs"]
mod capture;
#[path = "native_control_tui_constants.rs"]
mod constants;
#[path = "native_control_tui_interrupt.rs"]
mod interrupt;
#[path = "../federation/federation_tui_lease.rs"]
mod lease;
#[path = "native_control_tui_outcome.rs"]
mod outcome;
use constants::*;

pub async fn exercise(
    entry: &RunningNode,
    host: &Harness,
    assets: &Assets,
    target: RuntimeTarget,
    bystander: RuntimeTarget,
) {
    let baseline = observe::coherent_worker(host, target).await;
    let other_baseline = observe::coherent_worker(host, bystander).await;
    let selected = observe::current(entry, target, None).await;
    observe::count(&assets.target_marker, SECOND_DELIVERY).await;
    observe::count(&assets.other_marker, usize::default()).await;
    // The terminal running the real TUI is independent of the federated catalogue.
    let harness = Harness::start().await;
    let driver = harness.create_cat(DRIVER_NAME).await;
    let runtime = runtime_ref_from_view(&driver);
    let client = harness.client();
    let grant = client
        .acquire_writer(
            runtime,
            DRIVER_NAME,
            Some(yolo_runtime::lease::constants::MAX_TTL_MS),
        )
        .await
        .unwrap();
    let renewal = lease::Renewal::start(client.clone(), runtime, grant.clone());
    client
        .resize(
            runtime,
            grant.generation,
            &grant.writer_capability,
            COLUMNS,
            ROWS,
        )
        .await
        .unwrap();
    let mut output = client
        .attach(
            driver.manifest.terminal_id,
            yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
        )
        .await
        .unwrap();
    let mut screen = vt100::Parser::new(ROWS, COLUMNS, SCROLLBACK);
    screen.process(&output.initial.snapshot.as_ref().unwrap().data);
    let workspace = tempfile::tempdir().unwrap();
    let binary = crate::bazel_runfile(&std::env::var(BINARY_ENV).unwrap()).unwrap();
    let (launch, complete) =
        capture::launch(&binary, entry, &workspace.path().canonicalize().unwrap());
    send(&client, runtime, &grant, launch.as_bytes()).await;
    capture::wait_screen(&mut output, &mut screen, |text| {
        text.contains(TARGET_NAME) && text.contains(OTHER_NAME)
    })
    .await;
    send(&client, runtime, &grant, INVENTORY).await;
    send(
        &client,
        runtime,
        &grant,
        format!("/{}\r", selected.cursor().target().address).as_bytes(),
    )
    .await;
    capture::wait_screen(&mut output, &mut screen, |text| {
        text.contains(SINGLE_AGENT_HEADER)
    })
    .await;
    send(&client, runtime, &grant, DETAILS).await;
    let identity = selected.cursor().target().address.to_string();
    capture::wait_screen(&mut output, &mut screen, |text| {
        text.contains(SINGLE_AGENT_HEADER) && text.contains(&identity)
    })
    .await;
    send(&client, runtime, &grant, ESCAPE).await;
    send(&client, runtime, &grant, INTERRUPT).await;
    capture::wait_screen(&mut output, &mut screen, |text| text.contains(TAKE_CONTROL)).await;
    observe::count(&assets.target_marker, SECOND_DELIVERY).await;
    let Reply::LeaseObservation { lease } = entry
        .client()
        .await
        .call(
            target.address.hub_id,
            Command::ObserveLease { target },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(lease.is_none());
    send(&client, runtime, &grant, ENTER).await;
    let writing = format!("{TARGET_NAME}{CONTROL_SUFFIX}");
    capture::wait_screen(&mut output, &mut screen, |text| {
        text.lines()
            .last()
            .is_some_and(|line| line.contains(&writing))
    })
    .await;
    // Sidebar focus keeps the bound terminal and its authenticated writer intent live.
    send(&client, runtime, &grant, INVENTORY).await;
    capture::wait_screen(&mut output, &mut screen, |text| {
        text.contains(SINGLE_AGENT_HEADER)
            && text.contains(TARGET_NAME)
            && text.contains(INTERRUPT_HINT)
    })
    .await;
    interrupt::deliver(
        &client,
        runtime,
        &grant,
        &mut output,
        &mut screen,
        assets,
        (TARGET_NAME, INTERRUPT_HINT),
    )
    .await;
    let expected = SECOND_DELIVERY.checked_add(EXTRA_DELIVERY).unwrap();
    observe::count(&assets.target_marker, expected).await;
    observe::count(&assets.other_marker, usize::default()).await;
    observe::identities_unchanged(host, target, &baseline).await;
    observe::identities_unchanged(host, bystander, &other_baseline).await;
    let after_interrupt = observe::current(entry, target, None).await;
    assert_eq!(
        after_interrupt.cursor().target(),
        selected.cursor().target()
    );
    send(&client, runtime, &grant, QUIT).await;
    capture::wait_screen(&mut output, &mut screen, |text| text.contains(&complete)).await;
    observe::count(&assets.target_marker, expected).await;
    observe::count(&assets.other_marker, usize::default()).await;
    let grant = renewal.stop().await;
    client.release_writer(&grant, runtime).await.unwrap();
    output.detach().await.unwrap();
    client
        .terminate(driver.manifest.terminal_id, false)
        .await
        .unwrap();
}

async fn send(
    client: &yolo_client::Client,
    target: RuntimeRef,
    grant: &WriterLeaseGrant,
    bytes: &[u8],
) {
    client
        .input(
            target,
            grant.generation,
            &grant.writer_capability,
            bytes.to_vec(),
        )
        .await
        .unwrap();
}
