import { describe, expect, test } from "bun:test";
import { ApiError } from "../browser/api";
import { CreationJobError } from "../browser/creationJob";
import {
  forgetIntentKey,
  intentKey,
  keepIntentKey,
} from "../browser/creationKey";

class MemoryStorage {
  private readonly values = new Map<string, string>();
  getItem(key: string): string | null {
    return this.values.get(key) ?? null;
  }
  setItem(key: string, value: string): void {
    this.values.set(key, value);
  }
  removeItem(key: string): void {
    this.values.delete(key);
  }
}

Object.defineProperty(globalThis, "sessionStorage", {
  value: new MemoryStorage(),
  configurable: true,
});

describe("browser creation intent key", () => {
  test("reuses a key only for an identical form", () => {
    const body = { machine: "m", cwd: "/tmp", name: "one" };
    const key = intentKey("shell", body);
    expect(intentKey("shell", { ...body })).toBe(key);
    expect(intentKey("shell", { ...body, cwd: "/var" })).not.toBe(key);
  });

  test("forgets a completed or rejected intent", () => {
    const body = { machine: "m", cwd: "/tmp", name: "two" };
    const key = intentKey("agent", { ...body, provider: "codex" });
    forgetIntentKey(key);
    expect(intentKey("agent", { ...body, provider: "codex" })).not.toBe(key);
  });

  test("keeps the key only when the outcome is uncertain", () => {
    expect(keepIntentKey(new CreationJobError("unknown", true))).toBe(true);
    expect(keepIntentKey(new CreationJobError("rejected", false))).toBe(false);
    expect(keepIntentKey(new ApiError(400, "invalid"))).toBe(false);
    expect(keepIntentKey(new ApiError(401, "token"))).toBe(true);
    expect(keepIntentKey(new ApiError(503, "unavailable"))).toBe(true);
    expect(keepIntentKey(new TypeError("network"))).toBe(true);
  });
});
