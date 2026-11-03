const BASE_MS = 1_000;
const MAX_MS = 15_000;

/** Exponential delay with a cap; the attempt counter resets after live output. */
export function reconnectDelay(attempt: number): number {
  return Math.min(BASE_MS * 2 ** Math.min(attempt, 4), MAX_MS);
}
