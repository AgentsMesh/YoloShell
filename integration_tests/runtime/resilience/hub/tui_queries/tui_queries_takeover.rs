use super::{Driver, RunningHub, constants::*, deadline};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::LeaseGrant;

pub(super) async fn wait_for_revoked_writer(driver: &mut Driver, name: &str, grant: LeaseGrant) {
    let holder = grant.fields().principal.client_id.to_string();
    let displayed_holder = &holder[..OBSERVED_HOLDER_CHARACTERS];
    let occupied = format!("{name}{INPUT_HELD}{displayed_holder}{TAKE_CONTROL_HINT}");
    let interrupted = format!("{name}{INPUT_INTERRUPTED}");
    let occupied_interrupted = format!("{occupied} | {INTERRUPTED_INPUT_HINT}");
    // Renewal may reject the old grant before the canary is dispatched. If
    // input was already attempted, its failed stream requires explicit resume.
    // Each presentation must be an exact, focused, live-terminal status line.
    driver
        .wait(|text| {
            text.lines().last().is_some_and(|line| {
                let status = line.trim_end();
                status == occupied || status == interrupted || status == occupied_interrupted
            })
        })
        .await;
    eprintln!(
        "terminal query revoked writer status: {:?}",
        driver.screen().contents().lines().last()
    );
}

pub(super) async fn assert_external_writer(hub: &RunningHub, grant: LeaseGrant) {
    let expected = grant.fields();
    let reply = hub
        .client()
        .await
        .call(
            hub.hub,
            Command::ObserveLease {
                target: expected.target,
            },
            deadline(),
        )
        .await
        .unwrap();
    let Reply::LeaseObservation {
        lease: Some(observed),
    } = reply
    else {
        panic!("external writer disappeared during rejected input: {reply:?}");
    };
    let actual = observed.fields();
    assert_eq!(actual.target, expected.target);
    assert_eq!(actual.generation, expected.generation);
    assert_eq!(actual.holder_id, expected.principal.client_id.to_string());
    assert_eq!(
        actual.controller_instance_id,
        Some(expected.principal.controller_instance_id.to_string())
    );
}
