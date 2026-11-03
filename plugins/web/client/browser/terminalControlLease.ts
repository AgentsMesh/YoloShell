import {
  clearPending,
  flush,
  leaseKnown,
  wants,
  type Effect,
  type Machine,
} from "./terminalControlState.js";

const MAX_PENDING = 64 * 1024;

function takeIfReady(machine: Machine): { machine: Machine; effects: Effect[] } {
  if (
    !wants(machine) ||
    machine.takeSent ||
    machine.lease === "controlled" ||
    !machine.ready ||
    !leaseKnown(machine) ||
    machine.lease === "acquiring"
  ) {
    return { machine, effects: [] };
  }
  return {
    machine: { ...machine, takeSent: true },
    effects: [{ kind: "take" }],
  };
}

function request(machine: Machine): { machine: Machine; effects: Effect[] } {
  if (machine.disposed || machine.lease === "controlled") {
    return { machine, effects: [] };
  }
  const next = { ...machine, autoTake: false, intent: "take" as const, takeSent: false };
  const taken = takeIfReady(next);
  return {
    machine: taken.machine,
    effects: [{ kind: "clearError" }, ...taken.effects, { kind: "render" }],
  };
}

function maybeAutoTake(machine: Machine): { machine: Machine; effects: Effect[] } {
  if (!machine.autoTake || !machine.ready || !leaseKnown(machine)) {
    return { machine, effects: [] };
  }
  if (machine.lease === "controlled") {
    return { machine: { ...machine, autoTake: false }, effects: [] };
  }
  if (machine.otherController) {
    return { machine: { ...machine, autoTake: false }, effects: [{ kind: "render" }] };
  }
  if (machine.lease === "acquiring") return { machine, effects: [] };
  return request({ ...machine, autoTake: false });
}

export function ready(machine: Machine): { machine: Machine; effects: Effect[] } {
  let next = { ...machine, ready: true };
  const effects: Effect[] = [];
  const auto = maybeAutoTake(next);
  next = auto.machine;
  effects.push(...auto.effects);
  const taken = takeIfReady(next);
  next = taken.machine;
  effects.push(...taken.effects, { kind: "render" });
  return { machine: next, effects };
}

export function lease(
  machine: Machine,
  message: { state?: unknown; controlled?: unknown; controller?: unknown; viewers?: unknown },
): { machine: Machine; effects: Effect[] } {
  const previous = machine.lease;
  const state = typeof message.state === "string" ? (message.state as Machine["lease"]) : previous;
  let next = {
    ...machine,
    lease: state,
    otherController: message.controlled === true && message.controller !== true,
    viewers: typeof message.viewers === "number" ? message.viewers : machine.viewers,
  };
  const effects: Effect[] = [];
  if (state === "controlled") {
    const hadIntent = wants(next) || next.autoTake;
    next = { ...next, autoTake: false, intent: "none", takeSent: false };
    effects.push({ kind: "clearError" });
    const flushed = flush(next);
    next = flushed.machine;
    effects.push(...flushed.effects);
    if (next.stream === "live") effects.push({ kind: "fit" });
    if (hadIntent) effects.push({ kind: "focus" });
  } else if (state === "observing" || state === "lost") {
    if (previous === "controlled" || previous === "acquiring") {
      next = { ...clearPending(next), intent: "none", takeSent: false };
    }
    if (wants(next) && !next.takeSent) {
      const taken = takeIfReady(next);
      next = taken.machine;
      effects.push(...taken.effects);
    } else if (next.autoTake) {
      const auto = maybeAutoTake(next);
      next = auto.machine;
      effects.push(...auto.effects);
    }
  }
  effects.push({ kind: "stdin", enabled: !next.disposed }, { kind: "render" });
  return { machine: next, effects };
}

export function controlError(machine: Machine, message: string): { machine: Machine; effects: Effect[] } {
  return {
    machine: { ...clearPending(machine), autoTake: false, intent: "none", takeSent: false },
    effects: [{ kind: "showError", message }, { kind: "render" }],
  };
}

export function input(machine: Machine, data: Uint8Array): { machine: Machine; effects: Effect[] } {
  if (machine.disposed || data.byteLength === 0) return { machine, effects: [] };
  if (machine.lease === "controlled" && machine.stream === "live") {
    return { machine, effects: [{ kind: "input", data }] };
  }
  if (machine.otherController) return { machine, effects: [{ kind: "render" }] };
  if (machine.bytes + data.byteLength > MAX_PENDING) return { machine, effects: [] };
  const buffered = {
    ...machine,
    pending: [...machine.pending, Uint8Array.from(data)],
    bytes: machine.bytes + data.byteLength,
  };
  return request(buffered);
}

export function requestControl(machine: Machine): { machine: Machine; effects: Effect[] } {
  if (machine.disposed || machine.lease === "controlled") return { machine, effects: [] };
  if (machine.takeSent) return { machine: { ...machine, intent: "take" }, effects: [] };
  return request(machine);
}

export function release(machine: Machine): { machine: Machine; effects: Effect[] } {
  if (machine.disposed) return { machine, effects: [] };
  return {
    machine: {
      ...clearPending(machine),
      autoTake: false,
      intent: "none",
      takeSent: false,
      lease: "released",
    },
    effects: [{ kind: "release" }, { kind: "render" }],
  };
}
