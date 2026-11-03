import type { SelectionIntent } from "./types.js";

const SELECTION_KEY = "yolo-selection";

/** Persist the user's selection across reloads. */
export function rememberSelection(
  kind: "agent" | "terminal",
  id: string,
  machine?: string,
): void {
  try {
    sessionStorage.setItem(SELECTION_KEY, JSON.stringify({ kind, id, machine }));
  } catch {
    // selection is only a convenience
  }
}

export function rememberedSelection(): SelectionIntent | null {
  try {
    const value = JSON.parse(
      sessionStorage.getItem(SELECTION_KEY) ?? "null",
    ) as SelectionIntent | null;
    return value?.kind && value.id ? value : null;
  } catch {
    return null;
  }
}
