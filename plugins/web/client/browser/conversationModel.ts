import type { Entry, Part } from "./types.js";

export function entryKey(entry: Entry): string {
  return JSON.stringify(entry.id ?? [entry.role, entry.timestamp, entry.parts?.[0]?.text]);
}

export function entryLabel(entry: Entry): string {
  switch (entry.role) {
    case "user":
      return "You";
    case "assistant":
      return "Agent";
    case "tool":
      return "Tool";
    case "runtime":
      return "Runtime";
    default:
      return "Entry";
  }
}

export function partText(part: Part): string {
  return part.text ?? part.summary ?? part.output ?? part.arguments ?? part.native_kind ?? "";
}

export function timeLabel(value: string | number | null | undefined): string {
  if (value === null || value === undefined || value === "") return "";
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return String(value).slice(0, 24);
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}
