import { api } from "./api.js";
import { byId } from "./dom.js";
import { selectionPayload } from "./payload.js";
import { selected } from "./appSelectors.js";
import { getState } from "./store.js";
import { terminalPlainText } from "./terminalLogText.js";

let query: URLSearchParams | null = null;

function note(text: string, kind = ""): void {
  const element = byId("terminal-log-note");
  element.textContent = text;
  element.className = `note ${kind}`;
}

async function load(): Promise<void> {
  if (!query) return;
  note("Loading terminal log…");
  try {
    const result = await api.terminalLog(query);
    const plain = terminalPlainText(result.data);
    byId("terminal-log-text").textContent = plain.text;
    const size =
      result.data.byteLength < 1024
        ? `${result.data.byteLength} B`
        : `${Math.round(result.data.byteLength / 1024)} KB`;
    note(
      `${size}${result.truncated || plain.truncated ? " · earlier output omitted" : ""}`,
    );
  } catch (error) {
    note(error instanceof Error ? error.message : "Could not read the terminal log", "bad");
  }
}

export function openTerminalLog(): void {
  const payload = selectionPayload(selected(getState())) as {
    machine?: unknown;
    terminal_id?: unknown;
    runtime_epoch?: unknown;
  };
  if (
    typeof payload.machine !== "string" ||
    typeof payload.terminal_id !== "string" ||
    typeof payload.runtime_epoch !== "string"
  ) {
    return;
  }
  const dialog = byId<HTMLDialogElement>("terminal-log-dialog");
  if (dialog.open) return;
  query = new URLSearchParams({
    machine: payload.machine,
    terminal_id: payload.terminal_id,
    runtime_epoch: payload.runtime_epoch,
  });
  byId("terminal-log-text").textContent = "";
  dialog.showModal();
  void load();
}

function close(): void {
  byId<HTMLDialogElement>("terminal-log-dialog").close();
}

export function bindTerminalLog(): void {
  byId("terminal-log").addEventListener("click", openTerminalLog);
  byId("terminal-log-refresh").addEventListener("click", () => void load());
  byId("terminal-log-close").addEventListener("click", close);
}
