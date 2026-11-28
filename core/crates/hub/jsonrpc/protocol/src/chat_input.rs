use serde_json::{Map, Value};
use url::Url;
use yolo_conversation_protocol::{ChatContent, file_resource_link};

/// A resource_link refers to a file already staged on the execution's origin.
/// Formatting the prompt never opens a filesystem path or fetches a URI.
pub(super) fn normalize(params: &mut Map<String, Value>) -> Option<()> {
    let prompt = params.get_mut("request")?.get_mut("prompt")?;
    if prompt.is_string() {
        return Some(());
    }
    let blocks: Vec<ChatContent<String>> = serde_json::from_value(prompt.take()).ok()?;
    let mut message = String::new();
    let mut after_link = false;
    for block in blocks {
        match block {
            ChatContent::Text { text } => {
                if after_link && !text.is_empty() && !text.starts_with('\n') {
                    message.push('\n');
                }
                if !text.is_empty() {
                    message.push_str(&text);
                    after_link = false;
                }
            }
            ChatContent::ResourceLink { uri, name, .. } => {
                let uri = Url::parse(&uri).ok()?;
                if uri.scheme() != "file"
                    || uri.host_str().is_some()
                    || uri.query().is_some()
                    || uri.fragment().is_some()
                {
                    return None;
                }
                let path = uri.to_file_path().ok()?;
                let link = file_resource_link(&name, &path)?;
                if !message.is_empty() && !message.ends_with('\n') {
                    message.push('\n');
                }
                message.push_str(&link);
                after_link = true;
            }
        }
    }
    *prompt = Value::String(message);
    Some(())
}
