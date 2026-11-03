import { finishedTerminal } from "./catalogue.js";
import { byId } from "./dom.js";
import { terminalOfSelection } from "./model.js";
import { terminalPayload } from "./payload.js";
import { inventory } from "./appSelectors.js";
import { getState } from "./store.js";
import { TerminalView } from "./terminalView.js";
import type { Selected } from "./types.js";

let current: { key: string; view: TerminalView } | null = null;

export function syncTerminalView(selected: Selected | null, takeControl = false): void {
  const terminal = selected ? terminalOfSelection(selected) : undefined;
  const visible = terminal !== undefined && !byId("terminal-view").hidden;
  if (!terminal || !visible) {
    closeTerminalView();
    return;
  }
  if (finishedTerminal(terminal)) {
    closeTerminalView();
    byId("terminal-host").hidden = true;
    byId("terminal-error").hidden = true;
    byId<HTMLButtonElement>("terminal-take").hidden = true;
    const note = byId("terminal-finished");
    note.hidden = false;
    const status = byId("terminal-input-state");
    status.textContent = "Finished";
    status.classList.remove("is-active");
    return;
  }
  byId("terminal-host").hidden = false;
  byId("terminal-finished").hidden = true;
  const payload = terminalPayload(terminal) as {
    machine?: unknown;
    terminal_id?: unknown;
    runtime_epoch?: unknown;
  };
  const machine =
    typeof payload.machine === "string"
      ? payload.machine
      : inventory(getState())?.machine;
  if (
    typeof machine !== "string" ||
    typeof payload.terminal_id !== "string" ||
    typeof payload.runtime_epoch !== "string"
  ) {
    closeTerminalView();
    return;
  }
  const key = `${machine}:${payload.terminal_id}:${payload.runtime_epoch}`;
  if (current?.key === key) {
    if (takeControl) current.view.takeControl();
    current.view.reconcileSize(terminal.runtime?.size?.cols, terminal.runtime?.size?.rows);
    return;
  }
  current?.view.dispose();
  current = {
    key,
    view: new TerminalView(
      {
        machine,
        terminal_id: payload.terminal_id,
        runtime_epoch: payload.runtime_epoch,
      },
      takeControl,
    ),
  };
  current.view.open();
}

export function closeTerminalView(): void {
  current?.view.dispose();
  current = null;
}
