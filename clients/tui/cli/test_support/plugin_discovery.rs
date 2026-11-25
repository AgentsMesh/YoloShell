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
        // The release image's Git 2.30 reads XDG config before cloning.
        fs::create_dir(directory.path().join("git")).unwrap();
        let rewrite = format!("url.file://{}.insteadOf", registry.display());
        git(
            directory.path(),
            &[
                "config",
                FILE_OPTION,
                GIT_CONFIG_FILE,
                &rewrite,
                DEFAULT_REGISTRY,
            ],
        );
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
            .env("YOLO_PLUGIN_REGISTRY_ROOT", self.account())
            .env("GIT_ALLOW_PROTOCOL", "file")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("XDG_CONFIG_HOME", self.directory.path())
            .env(
                "GIT_CONFIG_GLOBAL",
                self.directory.path().join(GIT_CONFIG_FILE),
            )
            .env_remove("GIT_CONFIG_COUNT")
            .env_remove("GIT_CONFIG_PARAMETERS")
            .env("GIT_TRACE", &self.trace)
            .output()
            .unwrap()
    }

    pub(super) fn account(&self) -> PathBuf {
        self.directory.path().join("state")
    }

    pub(super) fn publish_source(&self) {
        let spec_path = self
            .registry
            .join("Specs")
            .join(WEB)
            .join(format!("{LATEST}.json"));
        let mut spec: Value = serde_json::from_slice(&fs::read(&spec_path).unwrap()).unwrap();
        let source = Path::new(spec["source"]["repository"].as_str().unwrap());
        fs::create_dir(source).unwrap();
        fs::write(source.join(MANIFEST_FILE), spec["manifest"].to_string()).unwrap();
        git(source, GIT_INIT);
        git(source, &["add", MANIFEST_FILE]);
        git(source, GIT_COMMIT);
        git(source, &["tag", spec["source"]["tag"].as_str().unwrap()]);
        spec["source"]["commit"] = git(source, &["rev-parse", "HEAD"]).into();
        fs::write(spec_path, spec.to_string()).unwrap();
        git(&self.registry, GIT_ADD);
        git(&self.registry, GIT_COMMIT);
    }

    pub(super) fn link(&self, id: &str) {
        let path = self.directory.path().join(id);
        fs::create_dir(&path).unwrap();
        fs::write(path.join(MANIFEST_FILE), manifest(id, EARLIER).to_string()).unwrap();
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

fn git(repository: &Path, args: &[&str]) -> String {
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
    success(&command.output().unwrap()).trim().to_owned()
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
