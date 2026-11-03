import { inventory, scope } from "./appSelectors.js";
import { byId } from "./dom.js";
import { providerSupported, syncProviderSelect } from "./providerPicker.js";
import { getState } from "./store.js";

export type Mode = "shell" | "agent";

export function selectedHub() {
  const value = byId<HTMLSelectElement>("create-machine").value;
  return inventory(getState())?.hubs?.hubs?.find((item) => item.machine === value);
}

function supported(
  hub: {
    reachable?: boolean;
    capabilities?: { create?: boolean; agent_start?: boolean };
  },
  mode: Mode,
): boolean {
  if (hub.reachable === false) return false;
  if (mode === "agent") {
    return hub.capabilities?.agent_start !== false && providerSupported(hub);
  }
  return hub.capabilities?.create !== false;
}

export function syncProviders(): void {
  syncProviderSelect(byId<HTMLSelectElement>("create-provider"), selectedHub());
}

export function machineOptions(mode: Mode): void {
  const select = byId<HTMLSelectElement>("create-machine");
  const snapshot = getState();
  const entry = inventory(snapshot)?.machine;
  const hubs = [...(inventory(snapshot)?.hubs?.hubs ?? [])].sort((left, right) => {
    if (left.machine === entry) return -1;
    if (right.machine === entry) return 1;
    return (left.label ?? "").localeCompare(right.label ?? "");
  });
  const options = hubs.length
    ? hubs
    : [{ machine: inventory(snapshot)?.machine, label: "This machine", capabilities: {} }];
  select.replaceChildren(
    ...options.map((hub) => {
      const option = document.createElement("option");
      option.value = hub.machine ?? "";
      const label = hub.label?.trim() || "Machine";
      option.textContent = supported(hub, mode)
        ? label
        : `${label} · ${mode === "agent" ? "no agents" : "no shells"}`;
      option.disabled = !supported(hub, mode);
      return option;
    }),
  );
  const preferred = scope(snapshot) ?? inventory(snapshot)?.machine;
  if (preferred && options.some((hub) => hub.machine === preferred && supported(hub, mode))) {
    select.value = preferred;
  } else {
    const first = options.find((hub) => supported(hub, mode));
    if (first) select.value = first.machine ?? "";
  }
}

export function selectedUnsupported(mode: Mode): boolean {
  const value = byId<HTMLSelectElement>("create-machine").value;
  const hub = (inventory(getState())?.hubs?.hubs ?? []).find(
    (item) => item.machine === value,
  );
  return hub !== undefined && !supported(hub, mode);
}

export function syncMachine(
  mode: Mode,
  busy: boolean,
  message: (text: string, bad: boolean) => void,
): void {
  if (busy) return;
  const unsupported = selectedUnsupported(mode);
  byId<HTMLButtonElement>("create-submit").disabled = unsupported;
  if (unsupported) {
    message(
      mode === "agent" && !providerSupported(selectedHub())
        ? "No agent provider is available on this machine."
        : mode === "agent"
          ? "This machine cannot start agents yet. Pick another machine or create a shell."
          : "This machine cannot create shells yet.",
      true,
    );
  } else {
    message("", false);
  }
}
