pub mod cli_constants;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

pub struct Fixture {
    root: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let serial = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let root =
            std::env::temp_dir().join(format!("yolo-architecture-{}-{serial}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn write(&self, relative: &str, contents: impl AsRef<[u8]>) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub fn load(root: &Path) -> std::io::Result<Vec<yolo_architecture_foundation::SourceFile>> {
    fn visit(
        root: &Path,
        directory: &Path,
        files: &mut Vec<yolo_architecture_foundation::SourceFile>,
    ) -> std::io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|name| name == ".build") {
                    continue;
                }
                visit(root, &path, files)?;
            } else {
                files.push(yolo_architecture_foundation::SourceFile {
                    absolute: path.clone(),
                    relative: path
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                    text: fs::read_to_string(&path)?,
                });
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort_by(|left, right| left.relative.cmp(&right.relative));
    Ok(files)
}

pub fn directories(root: &Path) -> std::io::Result<Vec<String>> {
    fn visit(root: &Path, directory: &Path, result: &mut Vec<String>) -> std::io::Result<()> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path.is_dir() {
                result.push(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned(),
                );
                visit(root, &path, result)?;
            }
        }
        Ok(())
    }
    let mut result = Vec::new();
    visit(root, root, &mut result)?;
    result.sort();
    Ok(result)
}
