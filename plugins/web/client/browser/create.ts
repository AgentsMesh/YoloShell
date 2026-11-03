import { api } from "./api.js";
import { waitForJob } from "./creationJob.js";
import { forgetIntentKey, intentKey, keepIntentKey } from "./creationKey.js";
import {
  machineOptions,
  selectedHub,
  selectedUnsupported,
  syncMachine,
  syncProviders,
  type Mode,
} from "./createOptions.js";
import { refreshUntil } from "./createRefresh.js";
import { selected } from "./appSelectors.js";
import { byId } from "./dom.js";
import { terminalOfSelection } from "./model.js";
import { providerAvailable } from "./providerPicker.js";
import { getState } from "./store.js";
type Created = { machine?: string; terminal_id?: string };
type Handlers = {
  refresh: () => Promise<void>;
  created: (machine: string, terminalId: string) => void;
  started: (
    machine: string,
    terminalId: string,
    provider: string,
    name?: string,
    cwd?: string,
  ) => void;
};
let handlers: Handlers | null = null;
let mode: Mode = "shell";
let busy = false;

function open(next: Mode): void {
  mode = next;
  const dialog = byId<HTMLDialogElement>("create");
  byId("create-title").textContent = next === "agent" ? "New agent" : "New shell";
  byId("create-submit").textContent = next === "agent" ? "Start agent" : "Create shell";
  byId("create-provider-row").hidden = next !== "agent";
  const terminal = terminalOfSelection(selected(getState()));
  byId<HTMLInputElement>("create-cwd").value = terminal?.cwd?.trim() || "/tmp";
  byId<HTMLInputElement>("create-name").value = "";
  byId("create-message").textContent = "";
  byId("create-message").className = "note";
  byId<HTMLButtonElement>("create-submit").disabled = false;
  machineOptions(mode);
  syncProviders();
  syncMachine(mode, busy, message);
  dialog.showModal();
  byId<HTMLInputElement>("create-cwd").focus();
}
function close(): void {
  byId<HTMLDialogElement>("create").close();
}

async function submit(event: SubmitEvent): Promise<void> {
  event.preventDefault();
  if (busy || !handlers) return;

  const workspace = byId<HTMLSelectElement>("create-machine").value;
  const cwd = byId<HTMLInputElement>("create-cwd").value.trim();
  const name = byId<HTMLInputElement>("create-name").value.trim();
  const provider = byId<HTMLSelectElement>("create-provider").value;
  if (!cwd) {
    message("Directory is required", true);
    return;
  }
  if (mode === "agent" && !providerAvailable(selectedHub(), provider)) {
    message("That agent provider is not available on this machine", true);
    return;
  }
  const body = {
    ...(workspace ? { machine: workspace } : {}),
    cwd,
    ...(name ? { name } : {}),
    ...(mode === "agent" ? { provider } : {}),
  };
  const key = intentKey(mode, body);
  const payload = { ...body, idempotency_key: key };
  busy = true;
  byId<HTMLButtonElement>("create-submit").disabled = true;
  message(mode === "agent" ? "Starting agent…" : "Creating shell…", false);
  try {
    const reply = (await (mode === "agent" ? api.startAgent(payload) : api.createShell(payload))) as {
      job_id?: string;
      terminal?: Created;
    };
    const terminal = reply.job_id
      ? ((await waitForJob(reply.job_id)) as { terminal?: Created }).terminal
      : reply.terminal;
    if (!terminal?.machine || !terminal.terminal_id) {
      throw new Error("The Hub returned no terminal address");
    }
    forgetIntentKey(key);
    close();
    if (mode === "agent") {
      handlers.started(terminal.machine, terminal.terminal_id, provider, name || undefined, cwd);
    }
    // Select the creation result now; the catalogue may not report it yet.
    handlers.created(terminal.machine, terminal.terminal_id);
    await refreshUntil(terminal.machine, terminal.terminal_id, handlers.refresh);
  } catch (error) {
    if (!keepIntentKey(error)) forgetIntentKey(key);
    message(error instanceof Error ? error.message : "Creation failed", true);
  } finally {
    busy = false;
    byId<HTMLButtonElement>("create-submit").disabled = selectedUnsupported(mode);
  }
}
function message(text: string, bad: boolean): void {
  const note = byId("create-message");
  note.textContent = text;
  note.className = `note ${bad ? "bad" : ""}`;
}

export function bindCreate(next: Handlers): void {
  handlers = next;
  byId("new-shell").addEventListener("click", () => open("shell"));
  byId("new-agent").addEventListener("click", () => open("agent"));
  byId("create-cancel").addEventListener("click", close);
  byId("create-machine").addEventListener("change", () => {
    syncProviders();
    syncMachine(mode, busy, message);
  });
  byId<HTMLFormElement>("create-form").addEventListener("submit", (event) => void submit(event));
  byId<HTMLDialogElement>("create").addEventListener("close", () => {
    byId("create-message").textContent = "";
  });
}
