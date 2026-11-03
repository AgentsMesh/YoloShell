import { entryKey, partText } from "./conversationModel.js";
import type { Entry } from "./types.js";

function normalized(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

function entryText(entry: Entry): string {
  return normalized((entry.parts ?? []).map(partText).join(" "));
}

/**
 * A prompt whose delivery receipt is unknown is confirmed only by a user
 * entry that appeared after the send and carries the same text. Entries that
 * were already visible (an earlier identical prompt) cannot confirm it.
 */
export function promptVisible(
  entries: Entry[],
  baseline: ReadonlySet<string>,
  text: string,
): boolean {
  const wanted = normalized(text);
  if (!wanted) return false;
  return entries.some(
    (entry) =>
      entry.role === "user" &&
      !baseline.has(entryKey(entry)) &&
      entryText(entry) === wanted,
  );
}
