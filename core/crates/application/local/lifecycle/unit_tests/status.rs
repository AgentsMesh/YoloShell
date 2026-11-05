use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;
use yolo_local_lifecycle::status;
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
