import { beforeEach, describe, expect, test } from "bun:test";

class FakeNode {
  children: FakeNode[] = [];
  textContent = "";
  appendChild<T extends FakeNode>(child: T): T {
    this.children.push(child);
    return child;
  }
}

class FakeElement extends FakeNode {
  dataset: Record<string, string> = {};
  href = "";
  target = "";
  rel = "";
  constructor(readonly tag: string) {
    super();
  }
}

function installDocument(): void {
  (globalThis as any).document = {
    createElement: (tag: string) => new FakeElement(tag),
    createTextNode: (text: string) => {
      const value = new FakeNode();
      value.textContent = text;
      return value;
    },
    createDocumentFragment: () => new FakeElement("#fragment"),
  };
}

function tags(value: FakeNode): string[] {
  return [
    ...(value instanceof FakeElement ? [value.tag] : []),
    ...value.children.flatMap(tags),
  ];
}

beforeEach(installDocument);

describe("Markdown DOM rendering", () => {
  test("renders lists and tables without a parser cycle", async () => {
    const { renderMarkdown } = await import("../browser/markdownRender");
    const fragment = renderMarkdown(
      ["- one", "- two", "", "| Name | Value |", "| --- | --- |", "| a | 1 |"].join("\n"),
    ) as unknown as FakeElement;
    expect(tags(fragment)).toEqual([
      "#fragment",
      "ul",
      "li",
      "li",
      "table",
      "thead",
      "tr",
      "th",
      "th",
      "tbody",
      "tr",
      "td",
      "td",
    ]);
  });

  test("turns only safe Markdown links into anchors", async () => {
    const { renderMarkdown } = await import("../browser/markdownRender");
    const fragment = renderMarkdown("[docs](https://example.com) [bad](javascript:alert(1))") as unknown as FakeElement;
    const links = fragment.children[0].children.filter(
      (child): child is FakeElement => child instanceof FakeElement && child.tag === "a",
    );
    expect(links).toHaveLength(1);
    expect(links[0].href).toBe("https://example.com");
    expect(links[0].rel).toBe("noopener noreferrer");
  });
});
