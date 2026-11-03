import { readFileSync } from "node:fs";
import { join } from "node:path";

export function readingToken(): string {
  const data = process.env.YOLO_PLUGIN_DATA ?? join(import.meta.dir, "..", ".data");
  return readFileSync(join(data, "token"), "utf8").trim();
}

export function sameToken(left: string, right: string): boolean {
  if (left.length !== right.length) return false;
  let difference = 0;
  for (let index = 0; index < left.length; index += 1) {
    difference |= left.charCodeAt(index) ^ right.charCodeAt(index);
  }
  return difference === 0;
}
