use super::super::{
    federation_support::{RunningNode, deadline},
    support::input,
};
use super::{assets::Assets, constants::*, launch, observe, requests, tui};
use crate::Harness;
use yolo_hub_agent_protocol::{CommandOutcome, FailureCode, ObservationCursor};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::LeaseMode;

pub async fn exercise(
    entry: &RunningNode,
    relay: &RunningNode,
    origin: &RunningNode,
    host: &Harness,
) {
    let assets = Assets::prepare(host.home.path());
    let target = launch::launch(origin, host, &assets, &assets.target_marker, TARGET_NAME).await;
    let other = launch::launch(origin, host, &assets, &assets.other_marker, OTHER_NAME).await;
    let local = observe::current(origin, target, None).await;
    let initial = observe::current(entry, target, None).await;
    let relayed = observe::current(relay, target, None).await;
    assert_eq!(initial.cursor(), local.cursor());
    assert_eq!(relayed.cursor(), local.cursor());
    let bystander = observe::current(entry, other, None).await;
    assert_ne!(bystander.cursor().target(), initial.cursor().target());
    let before = observe::coherent_worker(host, target).await;
    let other_before = observe::coherent_worker(host, other).await;
    observe::count(&assets.target_marker, usize::default()).await;
    observe::count(&assets.other_marker, usize::default()).await;

    let mut client = entry.client().await;
    let grant = requests::acquire_writer(&mut client, target, LeaseMode::Acquire {}).await;
    let original = requests::interrupt(initial.cursor());
    let delivered = requests::control(&mut client, original.clone(), grant).await;
    assert_eq!(delivered.outcome(), CommandOutcome::Delivered {});
    observe::count(&assets.target_marker, FIRST_DELIVERY).await;
    // Preserve the entire semantic request, including its ID and observed cursor.
    // A new transport envelope must resolve the original receipt without replay.
    let duplicate = requests::control(&mut client, original.clone(), grant).await;
    assert_eq!(duplicate, delivered);
    observe::count(&assets.target_marker, FIRST_DELIVERY).await;
    observe::count(&assets.other_marker, usize::default()).await;

    let stale = ObservationCursor::new(
        initial.cursor().target(),
        initial.cursor().stream(),
        initial
            .cursor()
            .sequence()
            .checked_add(CURSOR_STEP)
            .unwrap(),
    )
    .unwrap();
    let rejected = requests::control(&mut client, requests::interrupt(stale), grant).await;
    assert_eq!(
        rejected.outcome(),
        CommandOutcome::Failed {
            code: FailureCode::PreconditionsChanged
        }
    );
    observe::count(&assets.target_marker, FIRST_DELIVERY).await;

    // Make the same live provider publish a real native CwdChanged hook. This
    // advances the source witness while preserving Agent execution and runtime.
    let command = format!(
        "{} {}\r",
        crate::fake_agent_constants::PACKAGED_TUI_CWD_COMMAND,
        serde_json::to_string(assets.next.to_str().unwrap()).unwrap()
    );
    client
        .call(
            target.address.hub_id,
            input(grant, command.as_bytes()),
            deadline(),
        )
        .await
        .unwrap();
    let changed = observe::current(entry, target, Some(initial.cursor())).await;
    assert_eq!(changed.cursor().target(), initial.cursor().target());
    assert_eq!(
        changed
            .fields()
            .working_directory
            .as_ref()
            .unwrap()
            .as_str(),
        assets.next.to_str().unwrap()
    );
    let rejected =
        requests::control(&mut client, requests::interrupt(initial.cursor()), grant).await;
    assert_eq!(
        rejected.outcome(),
        CommandOutcome::Failed {
            code: FailureCode::PreconditionsChanged
        }
    );
    observe::count(&assets.target_marker, FIRST_DELIVERY).await;

    let mut origin_client = origin.client().await;
    let replacement = requests::acquire_writer(
        &mut origin_client,
        target,
        LeaseMode::TakeControl {
            observed_generation: grant.fields().generation,
        },
    )
    .await;
    assert!(replacement.fields().generation > grant.fields().generation);
    let current = observe::current(entry, target, None).await;
    let stale_writer =
        requests::control(&mut client, requests::interrupt(current.cursor()), grant).await;
    requests::not_delivered(stale_writer);
    observe::count(&assets.target_marker, FIRST_DELIVERY).await;
    let fresh = requests::control(
        &mut client,
        requests::interrupt(current.cursor()),
        replacement,
    )
    .await;
    assert_eq!(fresh.outcome(), CommandOutcome::Delivered {});
    observe::count(&assets.target_marker, SECOND_DELIVERY).await;
    observe::count(&assets.other_marker, usize::default()).await;

    observe::identities_unchanged(host, target, &before).await;
    observe::identities_unchanged(host, other, &other_before).await;
    let still_other = observe::current(entry, other, None).await;
    assert_eq!(still_other.cursor(), bystander.cursor());
    client
        .call(
            target.address.hub_id,
            Command::Release { grant: replacement },
            deadline(),
        )
        .await
        .unwrap();
    tui::exercise(entry, host, &assets, target, other).await;
    #[cfg(target_os = "macos")]
    super::mac::exercise(entry, host, &assets, target, other).await;
}
