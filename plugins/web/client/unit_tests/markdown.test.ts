import { describe, expect, test } from "bun:test";
import { parseInline, parseMarkdown } from "../browser/markdown";

describe("chat markdown blocks", () => {
  test("separates headings, fenced code, lists, quotes and tables", () => {
    const blocks = parseMarkdown(
      [
        "## Result",
        "",
        "```sh",
        "echo hello",
        "```",
        "",
        "- one",
        "- two",
        "",
        "> quoted",
        "",
        "| Name | Value |",
        "| --- | --- |",
        "| a | 1 |",
      ].join("\n"),
    );
    expect(blocks.map((block) => block.kind)).toEqual([
      "heading",
      "code",
      "list",
      "quote",
      "table",
    ]);
    expect(blocks[1]).toMatchObject({ kind: "code", language: "sh", text: "echo hello" });
    expect(blocks[2]).toMatchObject({ kind: "list", ordered: false, items: ["one", "two"] });
    expect(blocks[4]).toMatchObject({ kind: "table", head: ["Name", "Value"], rows: [["a", "1"]] });
  });

  test("keeps prose line breaks and ordered list markers", () => {
    expect(parseMarkdown("first\nsecond")).toEqual([
      { kind: "paragraph", text: "first\nsecond" },
    ]);
    expect(parseMarkdown("1. one\n2. two")[0]).toMatchObject({
      kind: "list",
      ordered: true,
      items: ["one", "two"],
    });
  });
});

describe("chat markdown inline", () => {
  test("parses emphasis, code and http links without interpreting HTML", () => {
    expect(parseInline("**bold** and `code` and [docs](https://example.com)")).toEqual([
      { kind: "strong", text: "bold" },
      { kind: "text", text: " and " },
      { kind: "code", text: "code" },
      { kind: "text", text: " and " },
      { kind: "link", text: "docs", href: "https://example.com" },
    ]);
    expect(parseInline("<img src=x onerror=alert(1)>")).toEqual([
      { kind: "text", text: "<img src=x onerror=alert(1)>" },
    ]);
    expect(parseInline("[click](javascript:alert(1))")).toEqual([
      { kind: "text", text: "[click](javascript:alert(1))" },
    ]);
    expect(parseInline("NEW_AGENT_OK and _em_")).toEqual([
      { kind: "text", text: "NEW_AGENT_OK and " },
      { kind: "em", text: "em" },
    ]);
  });
});
