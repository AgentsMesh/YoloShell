import { ApiError, forgetToken, saveToken } from "./api.js";
import { bindActions, closeDrawer } from "./actions.js";
import { clearAlerts } from "./alerts.js";
import { bindArchive } from "./archive.js";
import {
  filterChanged,
  notePendingAgent,
  scopeChanged,
} from "./catalogueController.js";
import { bindCreate } from "./create.js";
import { byId } from "./dom.js";
import { bindLifecycle } from "./lifecycle.js";
import { enableNotify, saveNotify, saveQuiet } from "./preferences.js";
import { renderSidebar, renderStatus, showGate } from "./render.js";
import { lock, setStatus } from "./requestController.js";
import {
  choose,
  chooseCreated,
  setLens,
} from "./selectionController.js";
import { dispatch, getState } from "./store.js";
import { bindTerminalLog } from "./terminalLog.js";

export function bind(handlers: {
  start: () => Promise<void>;
  refresh: () => Promise<void>;
}): void {
  bindActions();
  bindArchive();
  bindLifecycle();
  bindTerminalLog();
  bindCreate({
    refresh: handlers.refresh,
    created: (machine, terminalId) => chooseCreated(machine, terminalId),
    started: (machine, terminalId, provider, name, cwd) =>
      notePendingAgent(machine, terminalId, provider, name, cwd),
  });
  byId<HTMLFormElement>("gate-form").addEventListener("submit", async (event) => {
    event.preventDefault();
    const value = saveToken(byId<HTMLInputElement>("token").value);
    if (!value) {
      showGate("Enter the token from the plugin data directory.", "bad");
      return;
    }
    try {
      await handlers.start();
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        forgetToken();
        showGate("Token rejected.", "bad");
        return;
      }
      showGate(error instanceof Error ? error.message : "Could not connect.", "bad");
    }
  });
  // The gate markup paints before this module runs. Keep Connect disabled until
  // the submit handler exists so an early click cannot fall through to a native
  // form navigation and silently drop the token.
  byId<HTMLButtonElement>("gate-submit").disabled = false;
  byId("lock").addEventListener("click", lock);
  byId("notify").addEventListener("click", async () => {
    const state = getState();
    if (state.attention.notify) {
      dispatch({ type: "attention/notify", value: false });
      saveNotify(false);
      clearAlerts();
    } else {
      dispatch({ type: "attention/notify", value: true });
      saveNotify(true);
      const granted = await enableNotify();
      setStatus(
        granted
          ? "In-page and browser notifications on"
          : window.isSecureContext
            ? "In-page alerts on; browser notifications are blocked"
            : "In-page alerts on; browser notifications need HTTPS or localhost",
        granted ? "ok" : "",
      );
      renderStatus();
    }
    renderSidebar();
  });
  byId("quiet").addEventListener("click", () => {
    const quiet = !getState().attention.quiet;
    dispatch({ type: "attention/quiet", value: quiet });
    saveQuiet(quiet);
    if (quiet) clearAlerts();
    renderSidebar();
  });
  byId("alerts").addEventListener("click", (event) => {
    const target = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-id]");
    if (!target) return;
    target.closest(".alert")?.remove();
    choose(
      target.dataset.kind ?? "",
      target.dataset.id ?? "",
      false,
      target.dataset.machine,
    );
  });
  for (const id of ["needs-list", "agent-list", "terminal-list"]) {
    byId(id).addEventListener("click", (event) => {
      const target = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-id]");
      if (!target) return;
      choose(
        target.dataset.kind ?? "",
        target.dataset.id ?? "",
        false,
        target.dataset.machine,
      );
      if (target.dataset.lens === "terminal") setLens("terminal");
    });
  }
  byId("machine-list").addEventListener("click", (event) => {
    const target = (event.target as HTMLElement).closest<HTMLButtonElement>("button[data-scope]");
    if (!target) return;
    scopeChanged(target.dataset.scope || null);
  });
  byId("lens-chat").addEventListener("click", () => setLens("chat"));
  byId("lens-terminal").addEventListener("click", () => setLens("terminal"));
  byId<HTMLInputElement>("filter").addEventListener("input", (event) => {
    filterChanged((event.target as HTMLInputElement).value);
  });
  byId("menu").addEventListener("click", () => {
    const open = document.body.classList.toggle("sidebar-open");
    byId("menu").setAttribute("aria-expanded", String(open));
  });
  byId("app").addEventListener("click", (event) => {
    if (!document.body.classList.contains("sidebar-open")) return;
    if ((event.target as HTMLElement).closest("#sidebar, #menu")) return;
    closeDrawer();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") closeDrawer();
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "k") {
      event.preventDefault();
      byId<HTMLInputElement>("filter").focus();
    }
  });
}
