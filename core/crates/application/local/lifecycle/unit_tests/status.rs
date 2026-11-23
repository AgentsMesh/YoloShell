use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use tempfile::tempdir;
use tokio::time::Instant;
use yolo_build_image::BuildImage;
use yolo_hub_build_protocol::ServiceBuild;
use yolo_hub_jsonrpc_protocol::{Answer, Call};
use yolo_hub_jsonrpc_transport::{Framed, Incoming};
use yolo_hub_rpc_protocol::{Command, HUB_PROTOCOL_VERSION, Hello, Method, Reply};
use yolo_hub_state::HubState;
use yolo_local_lifecycle::{StatusReport, status};
use yolo_local_profile::LocalProfile;

#[tokio::test]
async fn status_reports_a_never_started_profile_as_stopped() {
    let directory = tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let profile_path = directory.path().join("profile.json");
    fs::write(
        &profile_path,
        format!(
            "{{\"schema_version\":1,\"state\":\"{}\",\"host_home\":\"{}\"}}\n",
            directory.path().join("state").display(),
            directory.path().join("home").display()
        ),
    )
    .unwrap();
    fs::set_permissions(&profile_path, fs::Permissions::from_mode(0o600)).unwrap();
    let profile = LocalProfile::load(&profile_path).unwrap();
    let report = status(&profile).await.unwrap();
    assert!(!report.hub.running);
    assert!(!report.host.running);
    assert!(report.hub.pid.is_none());
    assert!(report.host.pid.is_none());
    assert!(report.hub.image.is_none());
    assert!(!report.client_version.is_empty());
    assert!(!report.restart_needed);
}

async fn status_with_build(
    build: Option<ServiceBuild>,
    change_hello: impl FnOnce(&mut Hello),
) -> StatusReport {
    let directory = tempdir().unwrap();
    let base = directory.path().canonicalize().unwrap();
    let profile = LocalProfile::from_application_support(&base).unwrap();
    let state = HubState::open(&profile.state, &profile.host_home).unwrap();
    let budget = Duration::from_secs(10);
    let (listener, _publication) = yolo_paths::socket_activation::publish(
        state.socket_directory().unwrap(),
        &state.socket_path(),
        budget,
        "test Hub already running",
    )
    .await
    .unwrap();
    let peer = async {
        let deadline = Instant::now() + budget;
        let (stream, _) = tokio::time::timeout_at(deadline, listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut server = Framed::new(stream);
        let Incoming::Message(request) = server.receive::<Call>(deadline).await.unwrap() else {
            panic!()
        };
        assert!(matches!(request.fields().command, Command::Hello {}));
        let mut hello = Hello {
            hub_id: state.hub_id(),
            version: HUB_PROTOCOL_VERSION,
            methods: vec![Method::Hello, Method::GetBuild],
        };
        change_hello(&mut hello);
        let accepts_control =
            hello.version == HUB_PROTOCOL_VERSION && hello.valid_for_origin(state.hub_id());
        server
            .send(
                &Answer::new(&request, Reply::Hello { hello }).unwrap(),
                deadline,
            )
            .await
            .unwrap();
        if !accepts_control {
            return;
        }
        let Incoming::Message(request) = server.receive::<Call>(deadline).await.unwrap() else {
            panic!()
        };
        assert!(matches!(request.fields().command, Command::GetBuild {}));
        if let Some(build) = build {
            server
                .send(
                    &Answer::new(&request, Reply::Build { build }).unwrap(),
                    deadline,
                )
                .await
                .unwrap();
        }
    };
    let (report, ()) = tokio::join!(status(&profile), peer);
    report.unwrap()
}

#[tokio::test]
async fn a_live_hub_with_an_interrupted_build_probe_does_not_request_a_restart() {
    let report = status_with_build(None, |_| {}).await;
    assert!(report.hub.running && report.hub.reachable);
    assert_eq!(report.hub.protocol, Some(HUB_PROTOCOL_VERSION));
    assert!(report.hub.image.is_none());
    assert!(!report.live_images_consistent);
    assert!(!report.restart_needed);
}

#[tokio::test]
async fn a_confirmed_service_image_mismatch_requests_a_restart() {
    let executable = std::env::current_exe().unwrap().canonicalize().unwrap();
    let image = yolo_local_lifecycle::image(&executable).unwrap();
    let mut build = ServiceBuild {
        hub: image.clone(),
        host: image.clone(),
        file_helper: image,
    };
    let matching = status_with_build(Some(build.clone()), |_| {}).await;
    assert!(matching.live_images_consistent);
    assert!(!matching.restart_needed);
    build.host = BuildImage::new("a".repeat(64), 1).unwrap();
    let changed = status_with_build(Some(build), |_| {}).await;
    assert!(!changed.live_images_consistent);
    assert!(changed.restart_needed);
}

#[tokio::test]
async fn only_a_valid_hello_from_the_selected_hub_can_report_an_older_protocol() {
    let old_version = HUB_PROTOCOL_VERSION - 1;
    let old = status_with_build(None, |hello| hello.version = old_version).await;
    assert!(old.hub.running && !old.hub.reachable);
    assert_eq!(old.hub.protocol, Some(old_version));
    assert!(old.restart_needed);

    let wrong_hub = status_with_build(None, |hello| {
        hello.hub_id = yolo_hub_identity::HubId::new();
        hello.version = old_version;
    })
    .await;
    assert!(wrong_hub.hub.running && !wrong_hub.hub.reachable);
    assert_eq!(wrong_hub.hub.protocol, None);
    assert!(!wrong_hub.restart_needed);

    let malformed = status_with_build(None, |hello| {
        hello.methods.push(Method::Hello);
        hello.version = old_version;
    })
    .await;
    assert_eq!(malformed.hub.protocol, None);
    assert!(!malformed.restart_needed);
}
