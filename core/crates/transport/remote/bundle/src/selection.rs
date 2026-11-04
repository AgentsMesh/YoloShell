use crate::command::validate_channel;
use anyhow::Result;
use std::path::Path;
use yolo_distribution_publication_protocol::{Selection, decode_selection, validate_install_root};
use yolo_remote::{ChannelPurpose, MultiplexedSshConnection};

pub(crate) async fn read(
    connection: &dyn MultiplexedSshConnection,
    root: &Path,
) -> Result<Option<Selection>> {
    validate_install_root(root)?;
    let response = connection.read_bundle_selection(root).await?;
    validate_channel(connection, response.identity, ChannelPurpose::FileRead)?;
    response
        .value
        .map(|bytes| decode_selection(&bytes))
        .transpose()
}
