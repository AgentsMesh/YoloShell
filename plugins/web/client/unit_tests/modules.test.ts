import { describe, expect, test } from "bun:test";
import { readdir, readFile } from "node:fs/promises";
import { join } from "node:path";

const BROWSER = new URL("../browser/", import.meta.url).pathname;

describe("browser module graph", () => {
  test("every served browser module transpiles", async () => {
    const transpiler = new Bun.Transpiler({ loader: "ts" });
    const names = (await readdir(BROWSER)).filter((name) => name.endsWith(".ts"));
    expect(names.length).toBeGreaterThan(20);
    for (const name of names) {
      const source = await readFile(join(BROWSER, name), "utf8");
      expect(() => transpiler.transformSync(source), name).not.toThrow();
    }
  });
});
