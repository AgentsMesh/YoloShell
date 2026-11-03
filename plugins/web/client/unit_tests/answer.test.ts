import { describe, expect, test } from "bun:test";
import { answerResult } from "../browser/answer";

describe("answer receipt mapping", () => {
  test("clears the card only for accepted or delivered outcomes", () => {
    expect(answerResult({ state: "delivered" })).toEqual({
      text: "Answer delivered",
      ok: true,
      clear: true,
    });
    expect(answerResult({ state: "accepted" })).toEqual({
      text: "Answer accepted; awaiting result",
      ok: true,
      clear: true,
    });
  });

  test("keeps the card and reports failure or unknown", () => {
    expect(answerResult({ state: "failed", code: "preconditions_changed" })).toEqual({
      text: "Answer rejected; refresh the Agent",
      ok: false,
      clear: false,
    });
    expect(answerResult({ state: "failed", code: "adapter_rejected" })).toEqual({
      text: "Answer not delivered",
      ok: false,
      clear: false,
    });
    expect(answerResult({ state: "unknown" })).toEqual({
      text: "Answer outcome unknown; refresh the Agent",
      ok: false,
      clear: false,
    });
    expect(answerResult(undefined)).toEqual({
      text: "Answer outcome unknown; refresh the Agent",
      ok: false,
      clear: false,
    });
  });
});
