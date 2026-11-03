import { describe, expect, test } from "bun:test";
import { SnapshotCache } from "../server/snapshotCache";

describe("catalogue snapshot cache", () => {
  test("serves a fresh entry without another load", async () => {
    let now = 0;
    let loads = 0;
    const cache = new SnapshotCache<string>(1_500, () => now);
    expect(await cache.get(async () => (++loads, "one"))).toBe("one");
    now = 1_000;
    expect(await cache.get(async () => (++loads, "two"))).toBe("one");
    expect(loads).toBe(1);
  });

  test("coalesces concurrent loads and expires after the TTL", async () => {
    let now = 0;
    let loads = 0;
    let release!: (value: string) => void;
    const first = new Promise<string>((resolve) => {
      release = resolve;
    });
    const load = () => {
      loads += 1;
      return loads === 1 ? first : Promise.resolve(`load-${loads}`);
    };
    const cache = new SnapshotCache<string>(1_500, () => now);
    const left = cache.get(load);
    const right = cache.get(load);
    release("shared");
    expect(await left).toBe("shared");
    expect(await right).toBe("shared");
    expect(loads).toBe(1);
    now = 1_500;
    expect(await cache.get(load)).toBe("load-2");
  });

  test("clear prevents an in-flight read from repopulating the entry", async () => {
    let release!: (value: string) => void;
    let loads = 0;
    const pending = new Promise<string>((resolve) => {
      release = resolve;
    });
    const cache = new SnapshotCache<string>(1_500, () => 0);
    const oldRead = cache.get(() => (++loads, pending));
    cache.clear();
    release("old");
    expect(await oldRead).toBe("old");
    expect(await cache.get(async () => (++loads, "new"))).toBe("new");
    expect(loads).toBe(2);
  });

  test("a failed load is not cached", async () => {
    let loads = 0;
    const cache = new SnapshotCache<string>(1_500, () => 0);
    await expect(
      cache.get(async () => {
        loads += 1;
        throw new Error("unavailable");
      }),
    ).rejects.toThrow("unavailable");
    expect(await cache.get(async () => (++loads, "recovered"))).toBe("recovered");
    expect(loads).toBe(2);
  });
});
