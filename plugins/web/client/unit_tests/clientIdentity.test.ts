import { describe, expect, test } from "bun:test";
import { clientId } from "../browser/clientIdentity";

describe("browser client identity", () => {
  test("is a stable 128-bit tab identity", () => {
    expect(clientId()).toMatch(/^[0-9a-f]{32}$/);
    expect(clientId()).toBe(clientId());
  });
});
