use super::{BINARY_ENV, constants::*};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};
use tempfile::TempDir;

pub(super) struct Fixture {
    directory: TempDir,
    pub(super) registry: PathBuf,
    pub(super) trace: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let registry = directory.path().join("registry");
        let specs = registry.join("Specs").join(WEB);
        fs::create_dir_all(&specs).unwrap();
        for version in [EARLIER, LATEST] {
            let manifest = manifest(WEB, version);
            let spec = json!({
                (SCHEMA_FIELD): SCHEMA, "id": WEB, "version": version,
                "name": manifest["name"], "description": manifest["description"],
                "source": {
                    "repository": directory.path().join("absent-source"), "path": ".",
                    "tag": format!("v{version}"), "commit": COMMIT,
                },
                "manifest": manifest
            });
            fs::write(specs.join(format!("{version}.json")), spec.to_string()).unwrap();
        }
        for args in [GIT_INIT, GIT_ADD, GIT_COMMIT] {
            git(&registry, args);
        }
        let trace = directory.path().join("git.trace");
        Self {
            directory,
            registry,
            trace,
        }
    }

    pub(super) fn plugin(&self, args: &[&str]) -> Output {
        Command::new(std::env::var_os(BINARY_ENV).unwrap())
            .arg(PLUGIN)
            .args(args)
            .stdin(Stdio::null())
            .env(
                "YOLO_PLUGIN_REGISTRY_ROOT",
                self.directory.path().join("state"),
            )
            .env("GIT_ALLOW_PROTOCOL", "file")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_COUNT", "1")
            .env(
                "GIT_CONFIG_KEY_0",
                format!("url.file://{}.insteadOf", self.registry.display()),
            )
            .env("GIT_CONFIG_VALUE_0", DEFAULT_REGISTRY)
            .env("GIT_TRACE", &self.trace)
            .output()
            .unwrap()
    }

    pub(super) fn link(&self, id: &str) {
        let path = self.directory.path().join(id);
        fs::create_dir(&path).unwrap();
        fs::write(
            path.join("yolo-plugin.json"),
            manifest(id, EARLIER).to_string(),
        )
        .unwrap();
        success(&self.plugin(&[LINK, path.to_str().unwrap(), TRUST]));
    }

    pub(super) fn add_draft(&self, id: &str) {
        let directory = self.registry.join("Specs").join(id);
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("README.md"), "Draft plugin").unwrap();
        git(&self.registry, GIT_ADD);
        git(&self.registry, GIT_COMMIT);
    }

    pub(super) fn disconnect_registry(&self) {
        fs::rename(&self.registry, self.directory.path().join("unavailable")).unwrap();
    }
}

fn git(repository: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command
        .current_dir(repository)
        .args(args)
        .env("GIT_AUTHOR_NAME", "Plugin tests")
        .env("GIT_COMMITTER_NAME", "Plugin tests")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.test")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.test")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1");
    success(&command.output().unwrap());
}

fn manifest(id: &str, version: &str) -> Value {
    json!({
        (SCHEMA_FIELD): SCHEMA, "id": id, "name": id, "version": version,
        "description": "Discovery fixture", "platforms": ["linux", "macos"],
        "actions": [{"id": "check", "title": "Check", "command": ["false"]}],
        "api": {(PROTOCOL_FIELD): SCHEMA, "required_methods": [], "optional_methods": [], "features": []}
    })
}

pub(super) fn success(output: &Output) -> &str {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    std::str::from_utf8(&output.stdout).unwrap()
}
