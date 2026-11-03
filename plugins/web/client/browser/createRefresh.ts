import { ApiError } from "./api.js";
import { inventory } from "./appSelectors.js";
import { getState } from "./store.js";

/** Wait for the catalogue projection after a creation result. */
export async function refreshUntil(
  machine: string,
  terminalId: string,
  refresh: () => Promise<void>,
): Promise<void> {
  const deadline = Date.now() + 15_000;
  let nudged = 0;
  while (Date.now() < deadline) {
    if (Date.now() - nudged > 2_500) {
      nudged = Date.now();
      try {
        await refresh();
      } catch (error) {
        if (error instanceof ApiError && error.status === 401) throw error;
      }
    }
    const present = inventory(getState())?.terminals?.terminals?.some(
      (terminal) =>
        terminal.address?.machine === machine &&
        terminal.address?.terminal_id === terminalId,
    );
    if (present) return;
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
}
