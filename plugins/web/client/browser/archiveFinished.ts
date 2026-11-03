import { api } from "./api.js";
import {
  archiveHistoryNote,
  disposeArchiveTerminal,
  showArchiveTerminal,
} from "./archiveHistory.js";
import { byId } from "./dom.js";
import { terminalTitle } from "./model.js";
import type { Terminal } from "./types.js";

export type FinishedRow = Terminal & { finished?: boolean };

let pending: FinishedRow | null = null;

function button(): HTMLButtonElement {
  return byId<HTMLButtonElement>("archive-finish");
}

export function clearFinished(): void {
  pending = null;
  button().hidden = true;
  button().disabled = false;
}

/** Show the ended-but-not-archived detail instead of a history terminal. */
export function showFinished(item: FinishedRow): void {
  const epoch = item.runtime?.target?.runtime_epoch;
  pending = item;
  disposeArchiveTerminal();
  showArchiveTerminal(false);
  byId("archive-list").hidden = true;
  byId("archive-history").hidden = false;
  byId<HTMLButtonElement>("archive-back").hidden = false;
  button().hidden = false;
  button().disabled = !epoch;
  archiveHistoryNote(
    epoch
      ? `${terminalTitle(item)} · ended but not archived`
      : `${terminalTitle(item)} · ended; no runtime epoch is available`,
  );
}

export async function archivePending(refresh: () => Promise<void>): Promise<void> {
  const item = pending;
  const epoch = item?.runtime?.target?.runtime_epoch;
  const machine = item?.address?.machine;
  const terminalId = item?.address?.terminal_id;
  if (!item || !epoch || !machine || !terminalId) return;
  button().disabled = true;
  archiveHistoryNote("Archiving…");
  try {
    await api.lifecycle({
      machine,
      terminal_id: terminalId,
      runtime_epoch: epoch,
      action: "archive",
    });
    archiveHistoryNote("Archived");
    await refresh();
  } catch (error) {
    button().disabled = false;
    archiveHistoryNote(error instanceof Error ? error.message : "Archive failed");
  }
}
