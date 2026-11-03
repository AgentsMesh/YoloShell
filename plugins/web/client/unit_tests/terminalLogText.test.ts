import { describe, expect, test } from "bun:test";
import { terminalPlainText } from "../browser/terminalLogText";

const encode = (text: string): Uint8Array => new TextEncoder().encode(text);

describe("terminal log text", () => {
  test("strips SGR, OSC titles and cursor controls", () => {
    const result = terminalPlainText(
      encode(
        "\x1b]3008;start=abc\x07\x1b[1;32mhello\x1b[0m\r\n" +
          "\x1b[2K\x1b[1Gworld\r\n\x1b[?1049hfull\x1b[?1049l",
      ),
    );
    expect(result.text).toBe("hello\nworld\nfull");
    expect(result.truncated).toBe(false);
  });

  test("normalizes carriage returns and control bytes", () => {
    const result = terminalPlainText(encode("one\rtwo\x00three\tfour"));
    expect(result.text).toBe("one\ntwothree\tfour");
  });

  test("keeps the newest text when the display budget is exceeded", () => {
    const result = terminalPlainText(encode("a".repeat(2 * 1024 * 1024 + 10)));
    expect(result.truncated).toBe(true);
    expect(result.text.length).toBe(2 * 1024 * 1024);
  });
});
