import { describe, expect, test } from "bun:test";
import { entryKey } from "../browser/conversationModel";
import { promptVisible } from "../browser/promptReceipt";
import type { Entry } from "../browser/types";

function user(text: string, id?: number[]): Entry {
  return { id, role: "user", parts: [{ text }] };
}

describe("prompt receipt reconciliation", () => {
  test("confirms a matching user entry that appeared after the send", () => {
    const before = [user("earlier prompt", [1])];
    const baseline = new Set(before.map(entryKey));
    const after = [...before, user("  cargo   test\n", [2])];
    expect(promptVisible(after, baseline, "cargo test")).toBe(true);
  });

  test("does not confirm an earlier identical prompt", () => {
    const entries = [user("cargo test", [1])];
    const baseline = new Set(entries.map(entryKey));
    expect(promptVisible(entries, baseline, "cargo test")).toBe(false);
  });

  test("ignores assistant text and different prompts", () => {
    const baseline = new Set<string>();
    const entries: Entry[] = [
      { id: [3], role: "assistant", parts: [{ text: "cargo test" }] },
      user("cargo test --all", [4]),
    ];
    expect(promptVisible(entries, baseline, "cargo test")).toBe(false);
    expect(promptVisible(entries, baseline, "")).toBe(false);
  });
});
