import type { CreationIntent } from "./creationIntent";

export type PublicState = "running" | "unknown" | "done" | "error";

export type CreationTiming = {
  backoffMs: number[];
  maxAttempts: number;
  maxIntents: number;
  ttlMs: number;
};

export const DEFAULT_TIMING: CreationTiming = {
  backoffMs: [1_000, 2_000, 4_000, 8_000, 15_000, 30_000, 60_000],
  maxAttempts: 7,
  maxIntents: 512,
  ttlMs: 24 * 60 * 60_000,
};

export function recoverable(intent: CreationIntent): boolean {
  return ["prepared", "dispatched", "unconfirmed"].includes(intent.state);
}

export function publicState(intent: CreationIntent): PublicState {
  if (intent.state === "settled") return "done";
  if (intent.state === "failed") return "error";
  if (intent.state === "unconfirmed") return "unknown";
  return "running";
}

export function retryDelay(attempts: number, timing: CreationTiming): number {
  if (!timing.backoffMs.length) return 0;
  const index = Math.max(0, Math.min(attempts - 1, timing.backoffMs.length - 1));
  return timing.backoffMs[index];
}
