//! Set or remove labels on a durable agent run.

use crate::arguments::LabelArgs;
use crate::outcome::Outcome;
use anyhow::Result;
use yolo_hub_client::LocalClient;

pub async fn label(client: &mut LocalClient, args: LabelArgs) -> Result<Outcome> {
    let hub = client.hello().hub_id;
    let selector = crate::runs::selector(hub, &args.run)?;
    let run =
        yolo_agent_catalog::update_labels(client, hub, selector, &args.labels, &args.remove).await?;
    Ok(Outcome::success(
        args.output.json,
        serde_json::to_value(&run)?,
        yolo_agent_catalog::details(&run),
    ))
}
