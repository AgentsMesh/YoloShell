use crate::ProcessCommand;
use yolo_release_filesystem::TemporaryDirectory;

const DIRECTORY_PREFIX: &str = "yolo-process-resource-test-";
const DESCRIPTION: &str = "owned process resources";

pub(super) struct Resources {
    pub workspace: TemporaryDirectory,
    pub home: TemporaryDirectory,
    pub temporary: TemporaryDirectory,
    pub tool: TemporaryDirectory,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            workspace: TemporaryDirectory::create(DIRECTORY_PREFIX).unwrap(),
            home: TemporaryDirectory::create(DIRECTORY_PREFIX).unwrap(),
            temporary: TemporaryDirectory::create(DIRECTORY_PREFIX).unwrap(),
            tool: TemporaryDirectory::create(DIRECTORY_PREFIX).unwrap(),
        }
    }

    pub fn command(&self, executable: &str, arguments: &[&str]) -> ProcessCommand {
        ProcessCommand::new(
            executable,
            arguments
                .iter()
                .map(|argument| (*argument).to_owned())
                .collect(),
            self.workspace.handle(),
            self.home.handle(),
            self.temporary.handle(),
            DESCRIPTION,
        )
        .retain_directories([self.tool.handle()])
    }

    pub fn paths(&self) -> Vec<std::path::PathBuf> {
        [&self.workspace, &self.home, &self.temporary, &self.tool]
            .map(TemporaryDirectory::path)
            .into()
    }
}
