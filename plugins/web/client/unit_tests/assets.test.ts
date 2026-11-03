import { describe, expect, test } from "bun:test";
import { script } from "../server/assets";

describe("browser module assets", () => {
  test("serves every browser module without a hand-maintained list", async () => {
    for (const name of ["main", "terminalInput", "terminalReconnect", "terminalView"]) {
      expect(await script(name)).not.toBeNull();
    }
  });

  test("rejects unknown and traversal names", async () => {
    for (const name of ["missing", "../server/index", "/main", ""]) {
      expect(await script(name)).toBeNull();
    }
  });
});
