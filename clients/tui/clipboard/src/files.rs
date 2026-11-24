use anyhow::{Result, bail, ensure};
use std::path::{Path, PathBuf};
use url::Url;

pub(super) enum Item {
    File(PathBuf),
    Image { name: &'static str, bytes: Vec<u8> },
}

pub(super) fn file_urls(list: &str) -> Result<Vec<Item>> {
    list.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let uri = Url::parse(line)?;
            let path = uri
                .to_file_path()
                .map_err(|_| anyhow::anyhow!("Clipboard contains a non-local file URL"))?;
            Ok(Item::File(path))
        })
        .collect()
}

pub(super) fn file_path(input: &str) -> Result<PathBuf> {
    if input.is_empty() {
        bail!("Enter a file path");
    }
    if input.starts_with("file://") {
        return Url::parse(input)?
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("Choose a local file URL"));
    }
    if let Some(relative) = input.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return Ok(PathBuf::from(home).join(relative));
    }
    Ok(input.into())
}

/// Keep paths with spaces as one word. The text is inert when a plain shell owns the PTY.
pub(super) fn shell_word(path: &str) -> Result<String> {
    ensure!(
        Path::new(path).is_absolute() && !path.chars().any(char::is_control),
        "The Hub returned an invalid file reference"
    );
    Ok(
        if path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"/._-+".contains(&byte))
        {
            path.to_owned()
        } else {
            format!("'{}'", path.replace('\'', "'\\''"))
        },
    )
}

/// The shell expects words; Chat expects links that a native composer can submit as text.
#[derive(Clone, Copy)]
pub enum ReferenceStyle {
    Shell,
    Chat,
}

impl ReferenceStyle {
    pub(crate) fn format(self, paths: &[String]) -> Result<String> {
        let mut references = Vec::with_capacity(paths.len());
        for path in paths {
            references.push(match self {
                Self::Shell => shell_word(path)?,
                Self::Chat => {
                    let path = Path::new(path);
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .and_then(|name| yolo_conversation_protocol::file_resource_link(name, path))
                        .ok_or_else(|| {
                            anyhow::anyhow!("The Hub returned an invalid attachment path")
                        })?
                }
            });
        }
        Ok(references.join(match self {
            Self::Shell => " ",
            Self::Chat => "\n",
        }))
    }
}
