import type { Terminal } from "./types.js";

type Hub = {
  machine?: string;
  reachable?: boolean;
  capabilities?: { terminal_lifecycle?: boolean };
};

export type LifecycleActions = { terminate: boolean; archive: boolean };

/** End/Archive are offered only for an exact, reachable, capable machine. */
export function lifecycleActions(
  terminal: Terminal | null,
  hubs: Hub[] | undefined,
): LifecycleActions {
  if (!terminal || terminal.archived) return { terminate: false, archive: false };
  const machine = terminal.address?.machine;
  const hub = hubs?.find((value) => value.machine === machine);
  if (!machine || hub?.reachable === false || hub?.capabilities?.terminal_lifecycle !== true) {
    return { terminate: false, archive: false };
  }
  const status = terminal.runtime?.status;
  const fenced = typeof terminal.runtime?.target?.runtime_epoch === "string";
  return {
    terminate: status !== "exited" && status !== "lost" && fenced,
    archive: status === "exited" || status === "lost",
  };
}
