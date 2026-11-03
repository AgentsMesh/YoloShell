import { describe, expect, test } from "bun:test";
import { createHash } from "node:crypto";
import { vendorAsset } from "../server/vendor";

const manifest = (await Bun.file(new URL("../vendor/manifest.json", import.meta.url)).json()) as {
  packages: Array<{ files: Record<string, string> }>;
};

describe("vendored terminal assets", () => {
  test("serves only the fixed emulator files", async () => {
    for (const [name, type] of [
      ["xterm.js", "text/javascript"],
      ["xterm.css", "text/css"],
      ["addon-fit.js", "text/javascript"],
    ] as const) {
      const response = await vendorAsset(name);
      expect(response?.headers.get("content-type")).toContain(type);
      expect((await response?.arrayBuffer())?.byteLength).toBeGreaterThan(0);
    }
  });

  test("serves bytes matching every declared manifest hash", async () => {
    for (const pack of manifest.packages) {
      for (const [name, expected] of Object.entries(pack.files)) {
        if (!name.endsWith(".js") && !name.endsWith(".css")) continue;
        const response = await vendorAsset(name);
        const bytes = new Uint8Array((await response!.arrayBuffer()));
        expect(createHash("sha256").update(bytes).digest("hex")).toBe(expected);
      }
    }
  });

  test("rejects traversal and unknown files", async () => {
    for (const name of ["../xterm.js", "xterm.js.map", "other.js", "/xterm.js"]) {
      expect(await vendorAsset(name)).toBeNull();
    }
  });
});
