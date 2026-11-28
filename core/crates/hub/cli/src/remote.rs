use crate::{constants::*, options::Options};
use anyhow::{Result, ensure};
use std::io::Write;
use tokio::signal::unix::{SignalKind, signal};
use yolo_distribution_publication_protocol::{InspectionReceipt, PublicationReceipt};
use yolo_distribution_release_store::ReleaseStore;
use yolo_remote_provisioning_protocol::{MAX_BOOTSTRAP_DOCUMENT_BYTES, RemoteBootstrapRequest};

pub(crate) async fn run(options: Options) -> Result<()> {
    let root = options.install_root.as_ref().expect(INSTALL_ROOT_FIELD);
    if options.install_bundle {
        let request = yolo_distribution_publication_protocol::decode_request_argument(
            options.request_capnp.as_ref().expect(REQUEST_CAPNP_FIELD),
        )?;
        ensure!(root == request.install_root(), INSTALL_ROOT_MISMATCH);
        let canonical = std::fs::canonicalize(root)?;
        ensure!(canonical == request.install_root(), INSTALL_ROOT_MISMATCH);
        let store = ReleaseStore::open(root)?;
        ensure!(
            store.root() == request.install_root(),
            INSTALL_ROOT_MISMATCH
        );
        let installed = store.publish(
            request.stage(),
            request.descriptor(),
            request.expected().cloned(),
            request.operation_id(),
        )?;
        let receipt =
            PublicationReceipt::new(&request, store.root(), installed.selection().clone())?;
        return output(&yolo_distribution_publication_protocol::encode_receipt(
            &receipt,
        )?);
    }
    if options.inspect_install {
        yolo_distribution_publication_protocol::validate_install_root(root)?;
        let store = ReleaseStore::open_existing(root)?;
        let selected = store
            .as_ref()
            .map(ReleaseStore::inspect_existing)
            .transpose()?
            .flatten()
            .map(|installed| {
                (
                    installed.selection().clone(),
                    installed.descriptor().clone(),
                )
            });
        let actual_root = store.as_ref().map_or(root.as_path(), ReleaseStore::root);
        let receipt = InspectionReceipt::new(actual_root, selected)?;
        return output(&yolo_distribution_publication_protocol::encode_inspection(
            &receipt,
        )?);
    }
    let encoded = options.request_capnp.as_ref().expect(REQUEST_CAPNP_FIELD);
    let request: RemoteBootstrapRequest =
        yolo_capnp_argument::decode(encoded, MAX_BOOTSTRAP_DOCUMENT_BYTES)?;
    let directories = yolo_hub_remote_startup::RemoteDirectories {
        install_root: root.clone(),
        state: options.state.expect(STATE_FIELD),
        host_home: options.host_home.expect(HOST_HOME_FIELD),
    };
    if options.recover_remote_bootstrap {
        let deadline = tokio::time::Instant::now() + yolo_hub_local_startup::STARTUP_TIMEOUT;
        let recovery = yolo_hub_remote_startup::recover(&directories, &request, deadline).await?;
        let encoded = yolo_capnp_codec::encode(&recovery)?;
        ensure!(
            encoded.len() <= MAX_BOOTSTRAP_DOCUMENT_BYTES,
            DOCUMENT_TOO_LARGE
        );
        return output(&encoded);
    }
    let mut terminate = signal(SignalKind::terminate())?;
    let mut interrupt = signal(SignalKind::interrupt())?;
    let mut hangup = signal(SignalKind::hangup())?;
    let mut quit = signal(SignalKind::quit())?;
    let deadline = tokio::time::Instant::now() + yolo_hub_local_startup::STARTUP_TIMEOUT;
    let result = tokio::select! {
        result = yolo_hub_remote_startup::ensure_running(&directories, &request, deadline) => result,
        _ = terminate.recv() => Err(anyhow::anyhow!(BOOTSTRAP_CANCELLED)),
        _ = interrupt.recv() => Err(anyhow::anyhow!(BOOTSTRAP_CANCELLED)),
        _ = hangup.recv() => Err(anyhow::anyhow!(BOOTSTRAP_CANCELLED)),
        _ = quit.recv() => Err(anyhow::anyhow!(BOOTSTRAP_CANCELLED)),
    };
    yolo_process::wait_for_detached_termination_delivery(
        std::time::Instant::now() + BOOTSTRAP_RETIRE_TIMEOUT,
    )?;
    let encoded = yolo_capnp_codec::encode(&result?)?;
    ensure!(
        encoded.len() <= MAX_BOOTSTRAP_DOCUMENT_BYTES,
        DOCUMENT_TOO_LARGE
    );
    output(&encoded)
}

fn output(bytes: &[u8]) -> Result<()> {
    let mut output = std::io::stdout().lock();
    output.write_all(bytes)?;
    output.flush()?;
    Ok(())
}
