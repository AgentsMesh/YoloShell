import { describe, expect, test } from "bun:test";
import { watchRetry } from "../server/conversationActor";

describe("conversation watch retry policy", () => {
  test("backs off during the short retry window", () => {
    expect(watchRetry(1)).toEqual({ wait: 700, announce: false });
    expect(watchRetry(6)).toEqual({ wait: 1700, announce: false });
  });

  test("keeps probing and announces once after the window", () => {
    expect(watchRetry(7)).toEqual({ wait: 5000, announce: true });
    expect(watchRetry(100)).toEqual({ wait: 5000, announce: true });
  });
});
