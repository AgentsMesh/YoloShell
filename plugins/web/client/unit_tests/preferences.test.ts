import { describe, expect, test } from "bun:test";
import { documentTitle, shouldNotify } from "../browser/preferences";

describe("quiet attention alerts", () => {
  test("badges the title only when attention is pending and alerts are on", () => {
    expect(documentTitle(2, false)).toBe("(2) Yolo Web");
    expect(documentTitle(0, false)).toBe("Yolo Web");
    expect(documentTitle(2, true)).toBe("Yolo Web");
  });

  test("notifies only for new attention with alerts and notifications enabled", () => {
    expect(shouldNotify(0, 1, false, true)).toBe(true);
    expect(shouldNotify(1, 1, false, true)).toBe(false);
    expect(shouldNotify(0, 1, true, true)).toBe(false);
    expect(shouldNotify(0, 1, false, false)).toBe(false);
  });
});
