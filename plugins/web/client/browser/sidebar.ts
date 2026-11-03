import { attentionAgents, renderAttention } from "./attention.js";
import { alertNeeds } from "./alerts.js";
import { filter, inScope, inventory } from "./appSelectors.js";
import { timeLabel } from "./conversationModel.js";
import { byId } from "./dom.js";
import { renderMachines } from "./machines.js";
import { agentContext, agentTitle, matches } from "./model.js";
import { documentTitle, notifyNeeds, shouldNotify } from "./preferences.js";
import { renderSidebarRows } from "./sidebarRows.js";
import { getState } from "./store.js";

let lastNeeds = 0;

export function renderSidebar(): void {
  const snapshot = getState();
  const rows = renderSidebarRows(snapshot);
  alertNeeds(rows.allAgents);
  renderAttention(
    rows.allAgents.filter((agent) =>
      matches(`${agentTitle(agent)} ${agentContext(agent)}`),
    ),
  );
  renderMachines();
  const hubs = inventory(snapshot)?.hubs?.hubs ?? [];
  byId("hub-dot").className = `state-dot state-${snapshot.catalogue.connected ? "done" : "blocked"}`;
  const reachable = hubs.filter((hub) => hub.reachable).length;
  const attention = attentionAgents(rows.allAgents).length;
  const pendingInScope = rows.pending.filter((item) =>
    inScope(item.machine, snapshot),
  ).length;
  const agentTotal = rows.agents.length + pendingInScope;
  const agentLabel = `${agentTotal} agent${agentTotal === 1 ? "" : "s"}`;
  const shellLabel = `${rows.terminals.length} shell${rows.terminals.length === 1 ? "" : "s"}`;
  const machineLabelText = hubs.length > 1 ? ` · ${reachable}/${hubs.length} machines` : "";
  byId("hub-status").textContent = snapshot.catalogue.connected
    ? `${agentLabel} · ${shellLabel}${attention ? ` · ${attention} need you` : ""}${machineLabelText}`
    : "Disconnected";
  if (
    shouldNotify(
      lastNeeds,
      attention,
      snapshot.attention.quiet,
      snapshot.attention.notify,
    )
  ) {
    notifyNeeds(attention);
  }
  lastNeeds = attention;
  document.title = documentTitle(attention, snapshot.attention.quiet);
  const quietButton = byId<HTMLButtonElement>("quiet");
  quietButton.setAttribute("aria-pressed", String(snapshot.attention.quiet));
  quietButton.textContent = snapshot.attention.quiet ? "Quiet on" : "Quiet";
  const notifyButton = byId<HTMLButtonElement>("notify");
  notifyButton.setAttribute("aria-pressed", String(snapshot.attention.notify));
  notifyButton.textContent = snapshot.attention.notify ? "Notify on" : "Notify";
  byId("updated").textContent = snapshot.catalogue.updatedAt
    ? `Updated ${timeLabel(snapshot.catalogue.updatedAt)}`
    : "Not connected";
  const field = byId<HTMLInputElement>("filter");
  if (field.value !== filter(snapshot)) field.value = filter(snapshot);
}
