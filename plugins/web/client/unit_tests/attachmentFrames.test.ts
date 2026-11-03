import { describe, expect, test } from "bun:test";
import { FrameQueue } from "../../sdk/src/attachmentFrames";

function blob(bytes: number[]): Uint8Array {
  const header = Buffer.alloc(5);
  header[0] = 1;
  header.writeUInt32BE(bytes.length, 1);
  return Uint8Array.from(Buffer.concat([header, Buffer.from(bytes)]));
}

describe("terminal attachment frames", () => {
  test("preserves JSON and binary frame order across splits", async () => {
    const queue = new FrameQueue();
    const line = Buffer.from('{"jsonrpc":"2.0","method":"x"}\n');
    const binary = blob([1, 2, 3]);
    const all = Buffer.concat([line, Buffer.from(binary)]);
    queue.push(all.subarray(0, 7));
    queue.push(all.subarray(7, 30));
    queue.push(all.subarray(30));
    const first = await queue.take();
    const second = await queue.take();
    expect(first.kind).toBe("json");
    expect(second.kind).toBe("blob");
    if (second.kind === "blob") expect(Array.from(second.data)).toEqual([1, 2, 3]);
  });

  test("rejects malformed JSON and waits for complete frames", async () => {
    const queue = new FrameQueue();
    queue.push(Buffer.from('{"ok":'));
    let settled = false;
    const pending = queue.take().then((frame) => {
      settled = true;
      return frame;
    });
    await Bun.sleep(5);
    expect(settled).toBe(false);
    queue.push(Buffer.from("1}\n"));
    const frame = await pending;
    expect(frame.kind).toBe("json");

    const invalid = new FrameQueue();
    invalid.push(Buffer.from("not-json\n"));
    await expect(invalid.take()).rejects.toThrow("invalid JSON");
  });
});
