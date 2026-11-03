import { describe, expect, test } from "bun:test";
import { TerminalInputBinding } from "../browser/terminalInputBinding";

function harness(controlled: boolean) {
  const input: string[] = [];
  const control = {
    controlled,
    input: (data: Uint8Array) => input.push(new TextDecoder().decode(data)),
  };
  let data: ((value: string) => void) | null = null;
  const term = {
    onData: (callback: (value: string) => void) => (data = callback),
    onBinary: () => {},
  };
  new TerminalInputBinding(control as never).attach(term as never);
  return { control, input, type: (value: string) => data?.(value) };
}

describe("terminal input binding", () => {
  test("printable input while observing is user intent", () => {
    const { input, type } = harness(false);
    type("echo OK");
    expect(input).toEqual(["echo OK"]);
  });

  test("emulator query replies while observing are not forwarded", () => {
    const { input, type } = harness(false);
    type("\u001b[?1;2c");
    type("\u001b]10;rgb:ebeb/dbdb/b2b2\u001b\\");
    expect(input).toEqual([]);
  });

  test("an observing paste is dropped whole", () => {
    const { input, type } = harness(false);
    type("\u001b[200~");
    type("echo OK");
    type("\u001b[201~");
    expect(input).toEqual([]);
  });

  test("a one-chunk observing paste does not disable later input", () => {
    const { input, type } = harness(false);
    type("\u001b[200~echo OK\u001b[201~");
    type("echo NEXT");
    expect(input).toEqual(["echo NEXT"]);
  });

  test("a split observing paste clears on an end marker inside a chunk", () => {
    const { input, type } = harness(false);
    type("\u001b[200~echo ");
    type("OK\u001b[201~");
    type("echo NEXT");
    expect(input).toEqual(["echo NEXT"]);
  });

  test("taking control clears an unfinished observing paste", () => {
    const { control, input, type } = harness(false);
    type("\u001b[200~echo ");
    control.controlled = true;
    type("OK");
    expect(input).toEqual(["OK"]);
  });

  test("a controlled paste is forwarded", () => {
    const { input, type } = harness(true);
    type("\u001b[200~echo OK\u001b[201~");
    expect(input).toEqual(["\u001b[200~echo OK\u001b[201~"]);
  });

  test("emulator replies are forwarded once the lease is held", () => {
    const { input, type } = harness(true);
    type("\u001b[?1;2c");
    expect(input).toEqual(["\u001b[?1;2c"]);
  });
});
