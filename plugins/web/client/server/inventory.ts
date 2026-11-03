import { getCatalogue } from "./catalogueService";
import { machineCatalogue } from "./machineCatalogue";
export { catalogue, machineCatalogue } from "./machineCatalogue";

/** Ask the live catalogue observers for a fresh machine snapshot. */
export function invalidateInventory(): void {
  getCatalogue().invalidate();
}

/** Compatibility read for diagnostics; the browser uses the session stream. */
export async function inventory(): Promise<unknown> {
  const service = getCatalogue();
  const current = service.latest();
  if (current) return current;
  return new Promise((resolve) => {
    const stop = service.subscribe((event) => {
      if (event.kind !== "snapshot") return;
      stop();
      resolve(event.inventory);
    });
    setTimeout(() => {
      stop();
      resolve(service.latest() ?? { error: "catalogue unavailable" });
    }, 5_000);
  });
}
