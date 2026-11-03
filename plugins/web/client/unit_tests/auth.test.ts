import { expect, test } from "bun:test";
import { sameToken } from "../server/auth";

test("accepts only an equal token", () => {
  expect(sameToken("a".repeat(64), "a".repeat(64))).toBe(true);
  expect(sameToken("a".repeat(64), "b".repeat(64))).toBe(false);
  expect(sameToken("a".repeat(64), "a".repeat(63))).toBe(false);
});
