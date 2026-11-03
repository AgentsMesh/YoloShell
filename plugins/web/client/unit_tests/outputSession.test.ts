import { describe, expect, test } from "bun:test";
import { HubError } from "../../sdk/src/hubError";
import { historyUnavailable } from "../server/terminalSeed";

const failure = (code: string) => new HubError(-32000, "failed", { kind: "hub", code });

describe("terminal history fallback", () => {
  test("an unavailable history seed falls back to the screen snapshot", () => {
    expect(historyUnavailable(failure("outcome_unknown"))).toBe(true);
    expect(historyUnavailable(failure("unsupported"))).toBe(true);
    expect(historyUnavailable(failure("invalid_cursor"))).toBe(true);
    expect(historyUnavailable(new HubError(-32602, "Invalid params"))).toBe(true);
  });

  test("authorization and target failures are not retried as snapshots", () => {
    expect(historyUnavailable(failure("unauthorized"))).toBe(false);
    expect(historyUnavailable(failure("target_mismatch"))).toBe(false);
    expect(historyUnavailable(new Error("socket closed"))).toBe(false);
  });
});
