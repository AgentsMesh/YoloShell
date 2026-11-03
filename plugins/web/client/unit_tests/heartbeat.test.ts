import { describe, expect, test } from "bun:test";
import { HEARTBEAT_STALE_MS, heartbeatStale } from "../browser/heartbeat";

describe("terminal heartbeat", () => {
  test("a frame older than the stale bound is half-open", () => {
    const now = 1_000_000;
    expect(heartbeatStale(now - HEARTBEAT_STALE_MS - 1, now)).toBe(true);
    expect(heartbeatStale(now - HEARTBEAT_STALE_MS, now)).toBe(false);
    expect(heartbeatStale(now, now)).toBe(false);
  });
});
