use super::{
    constants::*,
    deadline,
    multihop_ingress::{Ingress, SharedClient},
};
use crate::hub::tui_command::TERMINALS_COMMAND;
use crate::{Harness, runtime_ref_from_view};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use yolo_hub_identity::HubId;
#[path = "../federation/federation_tui_lease.rs"]
mod lease;

pub async fn observe_entry(
    host: &Harness,
    ingress: &Ingress,
    shared: &SharedClient,
    hub: HubId,
    workspace: &Path,
    expected: &[&str],
    hidden: &[&str],
) {
    let view = host.create_cat(MULTIHOP_TUI_DRIVER_NAME).await;
    let target = runtime_ref_from_view(&view);
    let client = host.client();
    let grant = client
        .acquire_writer(
            target,
            MULTIHOP_TUI_DRIVER_NAME,
            Some(yolo_runtime::lease::constants::MAX_TTL_MS),
        )
        .await
        .unwrap();
    let renewal = lease::Renewal::start(client.clone(), target, grant.clone());
    client
        .resize(
            target,
            grant.generation,
            &grant.writer_capability,
            COLUMNS,
            ROWS,
        )
        .await
        .unwrap();
    let attached = client
        .attach(
            target.terminal_id,
            yolo_runtime::worker::FORCE_SNAPSHOT_AFTER_SEQUENCE,
        )
        .await
        .unwrap();
    let mut screen = vt100::Parser::new(ROWS, COLUMNS, SCROLLBACK);
    if let Some(snapshot) = attached.initial.snapshot.as_ref() {
        screen.process(&snapshot.data);
    }
    let output = crate::hub::tui_queries::driver::output::Output::start(attached);
    let binary = crate::bazel_runfile(&std::env::var(TUI_BINARY_ENV).unwrap()).unwrap();
    let workspace = workspace.canonicalize().unwrap();
    yolo_runtime::paths::directory::PrivateDirectory::ensure(&workspace).unwrap();
    let profile = workspace.join(MULTIHOP_PROFILE_FILE);
    ingress.write_profile(&profile);
    let profile = profile.canonicalize().unwrap();
    let account_key = ingress.account_key_path().canonicalize().unwrap();
    let client_key = shared.key_path.canonicalize().unwrap();
    for path in [
        profile.as_path(),
        account_key.as_path(),
        client_key.as_path(),
    ] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    }
    let nonce = uuid::Uuid::new_v4();
    let command = format!(
        "{CLIENT_STATE_ENV}='{}' '{}' {TERMINALS_COMMAND} {} '{}' {} '{}' {} '{}'; printf '%s%s\\n' '{}' '{nonce}'\n",
        quote(&workspace),
        quote(&binary),
        PROFILE_ARGUMENT,
        quote(&profile),
        ACCOUNT_KEY_ARGUMENT,
        quote(&account_key),
        CLIENT_KEY_ARGUMENT,
        quote(&client_key),
        MULTIHOP_TUI_COMPLETE_PREFIX,
    );
    client
        .input(
            target,
            grant.generation,
            &grant.writer_capability,
            command.into_bytes(),
        )
        .await
        .unwrap();
    output
        .wait(&mut screen, |text| {
            expected.iter().all(|name| text.contains(name))
        })
        .await
        .unwrap_or_else(|error| {
            panic!(
                "public TUI screen did not show {hub}: {error}; screen={}",
                screen.screen().contents()
            )
        });
    let text = screen.screen().contents();
    for name in hidden {
        assert!(
            !text.contains(name),
            "public TUI leaked hidden agent row: {name}"
        );
    }
    client
        .input(
            target,
            grant.generation,
            &grant.writer_capability,
            QUIT.to_vec(),
        )
        .await
        .unwrap();
    output
        .wait(&mut screen, |text| {
            text.contains(&format!("{MULTIHOP_TUI_COMPLETE_PREFIX}{nonce}"))
        })
        .await
        .unwrap();
    output.close().await;
    let grant = renewal.stop().await;
    client.release_writer(&grant, target).await.unwrap();
    client.terminate(target.terminal_id, false).await.unwrap();
}

fn quote(path: &Path) -> String {
    path.to_str().unwrap().replace('\'', QUOTE_ESCAPE)
}

const CLIENT_STATE_ENV: &str = "YOLO_TUI_STATE_DIRECTORY";
