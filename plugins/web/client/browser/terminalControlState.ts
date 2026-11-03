import {
  controlPresentation,
  type ControlState,
  type ControlPresentation,
} from "./terminalControlStatus.js";

export type StreamPhase = "connecting" | "seeding" | "live";
export type LeasePhase = "unknown" | "observing" | "acquiring" | "controlled" | "lost" | "released";
export type TakeIntent = "none" | "take";

export type Effect =
  | { kind: "take" }
  | { kind: "release" }
  | { kind: "input"; data: Uint8Array }
  | { kind: "stdin"; enabled: boolean }
  | { kind: "focus" }
  | { kind: "fit" }
  | { kind: "clearError" }
  | { kind: "showError"; message: string }
  | { kind: "render" };

export type Machine = {
  stream: StreamPhase;
  lease: LeasePhase;
  ready: boolean;
  intent: TakeIntent;
  takeSent: boolean;
  autoTake: boolean;
  otherController: boolean;
  viewers: number;
  pending: Uint8Array[];
  bytes: number;
  disposed: boolean;
};

export function initial(autoTake: boolean): Machine {
  return {
    stream: "connecting",
    lease: "unknown",
    ready: false,
    intent: "none",
    takeSent: false,
    autoTake,
    otherController: false,
    viewers: 1,
    pending: [],
    bytes: 0,
    disposed: false,
  };
}

export function leaseKnown(machine: Machine): boolean {
  return machine.lease !== "unknown";
}

export function wants(machine: Machine): boolean {
  return machine.intent === "take";
}

export function clearPending(machine: Machine): Machine {
  return { ...machine, pending: [], bytes: 0 };
}

export function flush(machine: Machine): { machine: Machine; effects: Effect[] } {
  if (machine.lease !== "controlled" || machine.stream !== "live") {
    return { machine, effects: [] };
  }
  const effects: Effect[] = machine.pending.map((data) => ({ kind: "input", data }));
  return { machine: clearPending(machine), effects };
}

export function reset(machine: Machine): { machine: Machine; effects: Effect[] } {
  return {
    machine: {
      ...machine,
      stream: "connecting",
      lease: "unknown",
      ready: false,
      intent: "none",
      takeSent: false,
      otherController: false,
      viewers: 1,
      pending: [],
      bytes: 0,
    },
    effects: [{ kind: "stdin", enabled: !machine.disposed }],
  };
}

export function setStream(
  machine: Machine,
  live: boolean,
  seeding: boolean,
): { machine: Machine; effects: Effect[] } {
  const stream: StreamPhase = seeding ? "seeding" : live ? "live" : "connecting";
  let next = { ...machine, stream };
  const effects: Effect[] = [{ kind: "stdin", enabled: !next.disposed }];
  if (live && !seeding) {
    const flushed = flush(next);
    next = flushed.machine;
    effects.push(...flushed.effects);
    if (next.lease === "controlled") effects.push({ kind: "fit" });
  }
  effects.push({ kind: "render" });
  return { machine: next, effects };
}

export function dispose(machine: Machine): Machine {
  return { ...clearPending(machine), disposed: true };
}

export function presentation(machine: Machine): ControlPresentation {
  return controlPresentation({
    state: machine.lease === "unknown" ? "observing" : (machine.lease as ControlState),
    live: machine.stream === "live",
    seeding: machine.stream === "seeding",
    wants: wants(machine),
    leaseKnown: leaseKnown(machine),
    otherController: machine.otherController,
    viewers: machine.viewers,
  });
}
