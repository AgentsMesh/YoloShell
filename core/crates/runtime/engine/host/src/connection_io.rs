use super::*;

pub(super) fn request_protocol_incompatible() -> ErrorBody {
    ErrorBody::protocol_incompatible(format!(
        "request envelope is incompatible; server supports protocol version {}",
        crate::model::PROTOCOL_VERSION
    ))
}

pub(super) fn client_protocol_incompatible(requested: u16) -> ErrorBody {
    ErrorBody::protocol_incompatible(format!(
        "client requested protocol {requested}; server supports {}",
        crate::model::PROTOCOL_VERSION
    ))
}

impl Host {
    pub(super) fn write_namespace_json<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        self.namespace.write_json(path, value)
    }

    pub(super) fn read_namespace_json<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T> {
        self.namespace.read_json(path)
    }

    pub(super) fn ensure_session_namespace(&self, terminal_id: Uuid) -> Result<()> {
        for path in [
            self.paths.session_dir(terminal_id),
            self.paths.terminal_dir(terminal_id),
            self.paths.events_dir(terminal_id),
            self.paths.logs_dir(terminal_id),
        ] {
            self.namespace.ensure_directory(&path)?;
        }
        Ok(())
    }
}
