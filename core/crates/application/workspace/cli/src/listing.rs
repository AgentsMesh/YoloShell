use crate::arguments::ListArgs;
use crate::commands::{WorkspaceCursor, call, print_page, unexpected};
use yolo_hub_client::LocalClient;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_workspace_protocol::{
    ListWorkspacesFields, ListWorkspacesRequest, WORKSPACE_PROTOCOL_SCHEMA_VERSION,
};

pub(crate) async fn list(
    client: &mut LocalClient,
    hub: HubId,
    args: ListArgs,
) -> anyhow::Result<()> {
    let after = match args.after {
        Some(WorkspaceCursor::Address(address)) => Some(address),
        Some(WorkspaceCursor::Id(id)) => {
            Some(crate::selector::address_of(client, hub, &id.to_string()).await?)
        }
        None => None,
    };
    let request = ListWorkspacesRequest::new(ListWorkspacesFields {
        schema_version: WORKSPACE_PROTOCOL_SCHEMA_VERSION,
        after,
        limit: args.limit,
    })?;
    let Reply::Workspaces { list } = call(client, hub, Command::ListWorkspaces { request }).await?
    else {
        return Err(unexpected());
    };
    print_page(&list, args.json)
}
