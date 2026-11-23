use super::super::{constants::*, fixture::Installation};
use std::{
    fs,
    io::Read,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};
use yolo_runtime::agent::{FencedDescendants, ProcessSource, RuntimeProcessSource};

pub(super) struct Launch {
    nonce: String,
    directory: PathBuf,
    pub command: String,
    pub complete: String,
}

impl Launch {
    pub fn new(installation: &Installation) -> Self {
        let nonce = uuid::Uuid::new_v4().to_string();
        let directory = installation.profile_path.parent().unwrap().join(&nonce);
        fs::DirBuilder::new()
            .mode(PRIVATE_DIRECTORY_MODE)
            .create(&directory)
            .unwrap();
        let command = format!(
            "{STTY_EXECUTABLE} -g > '{}'; printf '%s\\n' \"$?\" > '{}'; {CLIENT_STATE_ENV}='{}' {SHELL_EXECUTABLE} -c '{}' '{DRIVER_NAME}' '{nonce}' '{}' '{}' {LOCAL_PROFILE_ARGUMENT} '{}'; printf '%s\\n' \"$?\" > '{}'; {STTY_EXECUTABLE} -g > '{}'; printf '%s\\n' \"$?\" > '{}'; printf '%s%s\\n' '{COMPLETE_PREFIX}' '{nonce}'\n",
            quote(&directory.join(BEFORE_TERMIOS_FILE)),
            quote(&directory.join(BEFORE_STATUS_FILE)),
            quote(&installation.workspace),
            PROCESS_SCRIPT.replace('\'', QUOTE_ESCAPE),
            quote(&directory.join(PROCESS_FILE)),
            quote(&installation.tui),
            quote(&installation.profile_path),
            quote(&directory.join(TUI_STATUS_FILE)),
            quote(&directory.join(AFTER_TERMIOS_FILE)),
            quote(&directory.join(AFTER_STATUS_FILE))
        );
        Self {
            complete: format!("{COMPLETE_PREFIX}{nonce}"),
            command,
            nonce,
            directory,
        }
    }

    pub fn process(
        &self,
        installation: &Installation,
        root: crate::ProcessIdentity,
    ) -> crate::ProcessIdentity {
        self.require_success(BEFORE_STATUS_FILE);
        let hint = self.read(PROCESS_FILE);
        let mut fields = hint.lines();
        assert_eq!(fields.next(), Some(self.nonce.as_str()));
        let pid: u32 = fields.next().unwrap().parse().unwrap();
        assert!(fields.next().is_none());
        let process_root = yolo_runtime::process::ProcessIdentity {
            pid: root.pid,
            start_time: root.start_time,
        };
        let FencedDescendants::Stable(descendants) = RuntimeProcessSource::new([])
            .fenced_descendants(process_root)
            .unwrap()
        else {
            panic!("outer PTY process identity changed");
        };
        let expected = vec![
            installation.tui.to_str().unwrap().to_owned(),
            LOCAL_PROFILE_ARGUMENT.into(),
            installation.profile_path.to_str().unwrap().into(),
        ];
        let matches = descendants
            .iter()
            .filter(|process| {
                process.pid == pid
                    && Path::new(&process.executable) == installation.tui
                    && process.argv == expected
            })
            .collect::<Vec<_>>();
        let [observed] = matches.as_slice() else {
            panic!("launch hint does not identify the expected TUI image in the owned PTY scope");
        };
        let identity = crate::ProcessIdentity {
            pid: observed.pid,
            start_time: observed.start_time,
        };
        assert!(root.matches() && identity.matches());
        identity
    }

    pub fn completed(&self) {
        for status in [BEFORE_STATUS_FILE, TUI_STATUS_FILE, AFTER_STATUS_FILE] {
            self.require_success(status);
        }
        let before = self.read(BEFORE_TERMIOS_FILE);
        assert!(!before.is_empty());
        assert_eq!(self.read(AFTER_TERMIOS_FILE), before);
    }

    fn require_success(&self, name: &str) {
        let status: i32 = self.read(name).trim().parse().unwrap();
        assert_eq!(status, SUCCESS_STATUS, "launch command failed: {name}");
    }

    fn read(&self, name: &str) -> String {
        let mut bytes = String::new();
        fs::File::open(self.directory.join(name))
            .unwrap()
            .take(MAX_EVIDENCE_BYTES + OVERFLOW_BYTE)
            .read_to_string(&mut bytes)
            .unwrap();
        assert!(bytes.len() as u64 <= MAX_EVIDENCE_BYTES);
        bytes
    }
}

fn quote(path: &Path) -> String {
    path.to_string_lossy().replace('\'', QUOTE_ESCAPE)
}
