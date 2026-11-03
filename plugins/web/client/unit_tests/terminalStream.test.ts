import { describe, expect, test } from "bun:test";
import {
  begin,
  finish,
  initial,
  isSeeding,
  seed,
  settleLive,
} from "../browser/terminalStreamState";

describe("terminal stream state", () => {
  test("moves from connecting through seed to live", () => {
    const connecting = begin(initial());
    expect(connecting).toEqual({ phase: "connecting", generation: 1 });
    const seeding = seed(connecting);
    expect(seeding.phase).toBe("seeding");
    expect(isSeeding(seeding)).toBe(true);
    const result = settleLive(seeding, seeding.generation);
    expect(result.applied).toBe(true);
    expect(result.state.phase).toBe("live");
  });

  test("rejects a live write from an older seed", () => {
    const first = seed(initial());
    const second = seed(first);
    const result = settleLive(second, first.generation);
    expect(result.applied).toBe(false);
    expect(result.state).toBe(second);
    expect(isSeeding(second)).toBe(true);
  });

  test("rejects a live write after an exit", () => {
    const seeding = seed(initial());
    const ended = finish(seeding, "ended");
    expect(settleLive(ended, seeding.generation).applied).toBe(false);
    expect(isSeeding(ended)).toBe(false);
  });

  test("a reconnect invalidates pending live writes", () => {
    const seeding = seed(initial());
    const reconnecting = begin(seeding);
    expect(reconnecting.phase).toBe("connecting");
    expect(settleLive(reconnecting, seeding.generation).applied).toBe(false);
    expect(settleLive(reconnecting, reconnecting.generation).applied).toBe(false);
  });

  test("a failed stream is terminal until reopened", () => {
    const failed = finish(seed(initial()), "failed");
    expect(failed.phase).toBe("failed");
    expect(isSeeding(failed)).toBe(false);
    expect(settleLive(failed, failed.generation).applied).toBe(false);
  });
});
