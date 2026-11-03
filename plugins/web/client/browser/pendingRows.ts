import { matches, providerMarkKind, providerName, shortPath, terminalContext } from "./model.js";
import { row } from "./rows.js";
import type { PendingAgent } from "./pendingAgents.js";
import type { Terminal } from "./types.js";

/** A pending creation is a local intent; show it before its catalogue page lands. */
export function pendingRow(
  pending: PendingAgent,
  terminal: Terminal | undefined,
  selected: boolean,
): HTMLLIElement | null {
  const provider = providerName(pending.provider);
  const title = pending.name?.trim() || `${provider} starting`;
  const context = [
    provider,
    terminal ? terminalContext(terminal) : shortPath(pending.cwd ?? ""),
  ]
    .filter(Boolean)
    .join(" · ");
  if (!matches(`${title} ${context}`)) return null;
  return row(
    "terminal",
    pending.terminalId,
    pending.machine,
    title,
    context,
    providerMarkKind(pending.provider),
    { key: "working", label: "Starting" },
    selected,
  );
}
