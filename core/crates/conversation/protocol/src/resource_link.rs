use std::path::Path;
use url::Url;

/// Render a staged destination file as agent-readable Markdown. The label keeps
/// interactive providers from interpreting the paste as a standalone image drop.
pub fn file_resource_link(name: &str, path: &Path) -> Option<String> {
    if name.chars().any(char::is_control) || path.to_str()?.chars().any(char::is_control) {
        return None;
    }
    let url = Url::from_file_path(path).ok()?;
    let destination = url.as_str().replace('(', "%28").replace(')', "%29");
    let mut label = String::new();
    for character in name.chars() {
        if matches!(character, '\\' | '[' | ']' | '<' | '>' | '*' | '_' | '`') {
            label.push('\\');
        }
        label.push(character);
    }
    Some(format!("[{label}]({destination})"))
}
