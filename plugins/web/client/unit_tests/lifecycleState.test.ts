import { describe, expect, test } from "bun:test";
import { lifecycleActions } from "../browser/lifecycleState";
import type { Terminal } from "../browser/types";

const HUBS = [
  {
    machine: "m1",
    reachable: true,
    capabilities: { terminal_lifecycle: true },
  },
];

function terminal(status: string | undefined, extra: Partial<Terminal> = {}): Terminal {
  return {
    address: { machine: "m1", terminal_id: "t1" },
    runtime: status ? { status, target: { runtime_epoch: "e1" } } : undefined,
    ...extra,
  };
}

describe("browser lifecycle actions", () => {
  test("offers End for a live session and Archive for a finished one", () => {
    expect(lifecycleActions(terminal("running"), HUBS)).toEqual({
      terminate: true,
      archive: false,
    });
    expect(lifecycleActions(terminal("starting"), HUBS)).toEqual({
      terminate: true,
      archive: false,
    });
    expect(lifecycleActions(terminal("exited"), HUBS)).toEqual({
      terminate: false,
      archive: true,
    });
    expect(lifecycleActions(terminal("lost"), HUBS)).toEqual({
      terminate: false,
      archive: true,
    });
  });

  test("hides End until the runtime epoch is observable", () => {
    expect(
      lifecycleActions(
        {
          address: { machine: "m1", terminal_id: "t1" },
          runtime: { status: "starting" },
        },
        HUBS,
      ),
    ).toEqual({ terminate: false, archive: false });
  });

  test("hides actions without a capable, reachable machine", () => {
    expect(lifecycleActions(terminal("running"), [])).toEqual({
      terminate: false,
      archive: false,
    });
    expect(
      lifecycleActions(terminal("running"), [
        { machine: "m1", reachable: false, capabilities: { terminal_lifecycle: true } },
      ]),
    ).toEqual({ terminate: false, archive: false });
    expect(
      lifecycleActions(terminal("running"), [
        { machine: "m1", reachable: true, capabilities: { terminal_lifecycle: false } },
      ]),
    ).toEqual({ terminate: false, archive: false });
    expect(lifecycleActions(terminal("running", { archived: true }), HUBS)).toEqual({
      terminate: false,
      archive: false,
    });
  });
});
