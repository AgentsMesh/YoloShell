/** Terminal stream phases that are independent of the lease/input machine. */
export type StreamPhase = "connecting" | "seeding" | "live" | "ended" | "failed";

export type StreamState = {
  phase: StreamPhase;
  generation: number;
};

export function initial(): StreamState {
  return { phase: "connecting", generation: 0 };
}

/** A new socket generation invalidates every pending seed/live callback. */
export function begin(state: StreamState): StreamState {
  return { phase: "connecting", generation: state.generation + 1 };
}

/** A seed is starting; xterm is reset and input waits for the live marker. */
export function seed(state: StreamState): StreamState {
  return { phase: "seeding", generation: state.generation + 1 };
}

/**
 * Settle an asynchronous xterm write for the generation that saw `live`.
 * A reset, exit or reconnect in between makes the result stale.
 */
export function settleLive(
  state: StreamState,
  generation: number,
): { state: StreamState; applied: boolean } {
  if (state.phase !== "seeding" || state.generation !== generation) {
    return { state, applied: false };
  }
  return { state: { ...state, phase: "live" }, applied: true };
}

export function finish(state: StreamState, phase: "ended" | "failed"): StreamState {
  return { ...state, phase };
}

export function isSeeding(state: StreamState): boolean {
  return state.phase === "seeding";
}
