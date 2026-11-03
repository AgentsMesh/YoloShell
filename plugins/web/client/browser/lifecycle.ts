import { api } from "./api.js";
import { byId } from "./dom.js";
import { lifecycleActions } from "./lifecycleState.js";
import { terminalOfSelection, terminalTitle } from "./model.js";
import { inventory, selected } from "./appSelectors.js";
import { getState } from "./store.js";

type Action = "terminate" | "archive";
type Target = {
  machine: string;
  terminalId: string;
  runtimeEpoch?: string;
  name: string;
};
type Pending = Target & { action: Action };

let pending: Pending | null = null;
let ended: Target | null = null;
let busy = false;

function selectedTerminal() {
  return terminalOfSelection(selected(getState()));
}

function renderEnded(): void {
  const target = ended;
  if (target) {
    const present = inventory(getState())?.terminals?.terminals?.some(
      (terminal) =>
        terminal.address?.machine === target.machine &&
        terminal.address?.terminal_id === target.terminalId &&
        !terminal.archived,
    );
    if (!present) ended = null;
  }
  byId("lifecycle-note").hidden = ended === null;
  if (ended) byId("lifecycle-note-text").textContent = `${ended.name} ended.`;
}

export function renderLifecycle(): void {
  const actions = lifecycleActions(selectedTerminal(), inventory(getState())?.hubs?.hubs);
  byId<HTMLButtonElement>("terminal-end").hidden = !actions.terminate;
  byId<HTMLButtonElement>("terminal-archive").hidden = !actions.archive;
  renderEnded();
}

function ask(action: Action, target: Target): void {
  if (busy) return;
  pending = { ...target, action };
  byId("lifecycle-title").textContent =
    action === "terminate" ? "End session?" : "Archive session?";
  byId("lifecycle-copy").textContent =
    action === "terminate"
      ? `${target.name} and every program running in it will end.`
      : `${target.name} leaves the lists; its history remains on the machine.`;
  byId("lifecycle-confirm").textContent = action === "terminate" ? "End session" : "Archive";
  byId("lifecycle-message").textContent = "";
  byId<HTMLButtonElement>("lifecycle-confirm").disabled = false;
  byId<HTMLDialogElement>("lifecycle").showModal();
  byId<HTMLButtonElement>("lifecycle-cancel").focus();
}

function open(action: Action): void {
  const terminal = selectedTerminal();
  const actions = lifecycleActions(terminal, inventory(getState())?.hubs?.hubs);
  if (!terminal?.address?.machine || !terminal.address.terminal_id) return;
  if (action === "terminate" && !actions.terminate) return;
  if (action === "archive" && !actions.archive) return;
  ask(action, {
    machine: terminal.address.machine,
    terminalId: terminal.address.terminal_id,
    runtimeEpoch: terminal.runtime?.target?.runtime_epoch,
    name: terminal.name?.trim() || terminalTitle(terminal),
  });
}

function archiveEnded(): void {
  if (ended) ask("archive", ended);
}

async function submit(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  if (busy || !pending) return;
  const current = pending;
  busy = true;
  const confirm = byId<HTMLButtonElement>("lifecycle-confirm");
  confirm.disabled = true;
  byId("lifecycle-message").textContent =
    current.action === "terminate" ? "Ending…" : "Archiving…";
  try {
    await api.lifecycle({
      machine: current.machine,
      terminal_id: current.terminalId,
      runtime_epoch: current.runtimeEpoch,
      action: current.action,
    });
    if (current.action === "terminate") {
      ended = {
        machine: current.machine,
        terminalId: current.terminalId,
        runtimeEpoch: current.runtimeEpoch,
        name: current.name,
      };
    } else if (ended?.terminalId === current.terminalId) {
      ended = null;
    }
    pending = null;
    byId<HTMLDialogElement>("lifecycle").close();
    renderEnded();
  } catch (error) {
    byId("lifecycle-message").textContent =
      error instanceof Error ? error.message : "Lifecycle change failed";
  } finally {
    busy = false;
    confirm.disabled = false;
  }
}

export function bindLifecycle(): void {
  byId("terminal-end").addEventListener("click", () => open("terminate"));
  byId("terminal-archive").addEventListener("click", () => open("archive"));
  byId("lifecycle-archive-ended").addEventListener("click", archiveEnded);
  byId<HTMLFormElement>("lifecycle-form").addEventListener("submit", (event) =>
    void submit(event),
  );
  byId("lifecycle-cancel").addEventListener("click", () =>
    byId<HTMLDialogElement>("lifecycle").close(),
  );
  byId<HTMLDialogElement>("lifecycle").addEventListener("close", () => {
    // A queued close event from the End dialog can arrive after the ended
    // note reopened the same dialog for Archive; keep that target.
    if (byId<HTMLDialogElement>("lifecycle").open) return;
    pending = null;
    byId("lifecycle-message").textContent = "";
  });
}
