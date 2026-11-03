use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use yolo_hook_installation::{IntegrationStatus, inspect, install};
mod catalog {
    pub use yolo_claude_provider::INSTALL_SPEC as CLAUDE;
    pub use yolo_codex_provider::INSTALL_SPEC as CODEX;
    pub use yolo_gemini_provider::INSTALL_SPEC as GEMINI;
    pub use yolo_grok_provider::INSTALL_SPEC as GROK;
    pub use yolo_qwen_provider::INSTALL_SPEC as QWEN;
}
mod safety;
#[path = "../test_support/installation.rs"]
mod support;
use support::*;

#[test]
fn installation_preserves_exact_backup_and_is_idempotent() {
    for provider in [&catalog::CLAUDE, &catalog::CODEX] {
        let home = private_directory();
        let path = original(home.path(), provider);
        let preview = inspect(provider, home.path()).unwrap();
        let installed = install(provider, home.path(), &preview.revision).unwrap();
        assert!(installed.changed);
        assert_eq!(installed.preview.status, IntegrationStatus::Configured);
        assert_eq!(fs::read(installed.backup.unwrap()).unwrap(), ORIGINAL);
        let before = fs::read(&path).unwrap();
        let repeated = install(provider, home.path(), &installed.preview.revision).unwrap();
        assert!(!repeated.changed);
        assert!(repeated.backup.is_none());
        assert_eq!(fs::read(&path).unwrap(), before);
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & PERMISSION_MASK,
            PRIVATE
        );
    }
}

#[test]
fn a_missing_provider_root_and_hook_directory_are_both_created() {
    // Providers whose hook file lives in a subdirectory (Grok's
    // `~/.grok/hooks`) have not created their root before their CLI runs.
    let root = private_directory();
    let home = root.path().join("provider").join("hooks");
    let preview = inspect(&catalog::CLAUDE, &home).unwrap();
    assert_eq!(preview.status, IntegrationStatus::Missing);
    let installed = install(&catalog::CLAUDE, &home, &preview.revision).unwrap();
    assert!(installed.changed);
    assert_eq!(installed.preview.status, IntegrationStatus::Configured);
    assert!(home.join(catalog::CLAUDE.file_name).is_file());
    assert_eq!(
        fs::metadata(&home).unwrap().permissions().mode() & PERMISSION_MASK,
        DIRECTORY
    );
    assert_eq!(
        fs::metadata(home.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & PERMISSION_MASK,
        DIRECTORY
    );
}

#[test]
fn gemini_hooks_merge_into_the_settings_event_shape() {
    let home = private_directory();
    let path = home.path().join(catalog::GEMINI.file_name);
    fs::write(
        &path,
        br#"{"security":{"auth":{"selectedType":"gemini-api-key"}}}"#,
    )
    .unwrap();
    permissions(&path, PRIVATE);
    let preview = inspect(&catalog::GEMINI, home.path()).unwrap();
    let installed = install(&catalog::GEMINI, home.path(), &preview.revision).unwrap();
    assert!(installed.changed);
    assert_eq!(installed.preview.status, IntegrationStatus::Configured);
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(document["security"]["auth"]["selectedType"], "gemini-api-key");
    for event in [
        "SessionStart",
        "BeforeAgent",
        "AfterAgent",
        "BeforeTool",
        "AfterTool",
        "Notification",
        "SessionEnd",
    ] {
        let groups = document["hooks"][event].as_array().unwrap();
        let command = groups[0]["hooks"][0]["command"].as_str().unwrap();
        assert!(
            command.contains(catalog::GEMINI.hook_arguments),
            "{event}: {command}"
        );
    }
}

#[test]
fn gemini_disabled_hooks_are_reported_and_not_overwritten() {
    let home = private_directory();
    let path = home.path().join(catalog::GEMINI.file_name);
    let original = br#"{"hooksConfig":{"enabled":false}}"#;
    fs::write(&path, original).unwrap();
    permissions(&path, PRIVATE);
    let preview = inspect(&catalog::GEMINI, home.path()).unwrap();
    assert_eq!(preview.status, IntegrationStatus::Restricted);
    assert!(install(&catalog::GEMINI, home.path(), &preview.revision).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn missing_config_installs_without_fabricating_a_backup() {
    let home = private_directory();
    let preview = inspect(&catalog::CODEX, home.path()).unwrap();
    let installed = install(&catalog::CODEX, home.path(), &preview.revision).unwrap();
    assert!(installed.changed);
    assert!(installed.backup.is_none());
    assert!(installed.preview.requires_provider_trust);
}

#[test]
fn stale_preview_and_same_content_inode_replacement_are_rejected() {
    let home = private_directory();
    let provider = &catalog::CLAUDE;
    let path = original(home.path(), provider);
    let preview = inspect(provider, home.path()).unwrap();
    let moved = home.path().join("retained-original");
    fs::rename(&path, &moved).unwrap();
    original(home.path(), provider);
    assert!(install(provider, home.path(), &preview.revision).is_err());
    assert_eq!(fs::read(&path).unwrap(), ORIGINAL);
    let fresh = inspect(provider, home.path()).unwrap();
    fs::write(&path, b"{\"userChanged\":true}").unwrap();
    assert!(install(provider, home.path(), &fresh.revision).is_err());
}

#[test]
fn symlink_and_hardlink_configuration_never_redirect_writes() {
    for hardlink in [false, true] {
        let home = private_directory();
        let outside = home.path().join("outside.json");
        fs::write(&outside, ORIGINAL).unwrap();
        fs::set_permissions(&outside, fs::Permissions::from_mode(PRIVATE)).unwrap();
        let target = home.path().join(catalog::CODEX.file_name);
        if hardlink {
            fs::hard_link(&outside, &target).unwrap();
        } else {
            symlink(&outside, &target).unwrap();
        }
        assert!(inspect(&catalog::CODEX, home.path()).is_err());
        assert!(install(&catalog::CODEX, home.path(), "absent").is_err());
        assert_eq!(fs::read(&outside).unwrap(), ORIGINAL);
    }
}

#[test]
fn malformed_and_disabled_settings_are_not_overwritten() {
    let home = private_directory();
    let path = original(home.path(), &catalog::CLAUDE);
    fs::write(&path, b"{\"disableAllHooks\":true}").unwrap();
    let preview = inspect(&catalog::CLAUDE, home.path()).unwrap();
    assert_eq!(preview.status, IntegrationStatus::Restricted);
    assert!(install(&catalog::CLAUDE, home.path(), &preview.revision).is_err());
    fs::write(&path, b"malformed").unwrap();
    assert!(inspect(&catalog::CLAUDE, home.path()).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"malformed");
}

#[test]
fn legacy_backups_are_removed_and_new_backups_are_not_hook_files() {
    let home = private_directory();
    let provider = &catalog::GROK;
    let path = original(home.path(), provider);
    let legacy = home
        .path()
        .join("yolo-hooks-backup-deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef.json");
    fs::write(&legacy, fs::read(&path).unwrap()).unwrap();
    permissions(&legacy, PRIVATE);
    let preview = inspect(provider, home.path()).unwrap();
    let installed = install(provider, home.path(), &preview.revision).unwrap();
    assert!(!legacy.exists(), "legacy backup must not remain a hook file");
    let backup = installed.backup.unwrap();
    assert!(
        backup.to_string_lossy().ends_with(".json.bak"),
        "{}",
        backup.display()
    );
}
