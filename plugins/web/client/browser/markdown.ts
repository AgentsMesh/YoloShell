import type { Block, Inline } from "./markdownTypes.js";

const FENCE = /^\s*```([\w+#.-]*)\s*$/;
const RULE = /^\s*([-*_])(?:\s*\1){2,}\s*$/;
const HEADING = /^\s*(#{1,4})\s+(.*?)\s*#*\s*$/;
const ITEM = /^\s*(?:([-*+])|(\d+)[.)])\s+(.*)$/;
const QUOTE = /^\s*>\s?(.*)$/;
const TABLE_RULE = /^\s*\|?\s*:?-{3,}:?\s*(?:\|\s*:?-{3,}:?\s*)+\|?\s*$/;

function cells(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

function tableAt(lines: string[], index: number): Block | null {
  if (!lines[index].includes("|") || !TABLE_RULE.test(lines[index + 1] ?? "")) return null;
  const head = cells(lines[index]);
  const rows: string[][] = [];
  let next = index + 2;
  while (next < lines.length && lines[next].includes("|") && lines[next].trim()) {
    rows.push(cells(lines[next]));
    next += 1;
  }
  return { kind: "table", head, rows };
}

export function parseMarkdown(source: string): Block[] {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const blocks: Block[] = [];
  let index = 0;
  while (index < lines.length) {
    const line = lines[index];
    if (!line.trim()) {
      index += 1;
      continue;
    }
    const fence = FENCE.exec(line);
    if (fence) {
      let end = index + 1;
      while (end < lines.length && !FENCE.test(lines[end])) end += 1;
      blocks.push({
        kind: "code",
        language: fence[1] || undefined,
        text: lines.slice(index + 1, end).join("\n"),
      });
      index = end + 1;
      continue;
    }
    if (RULE.test(line)) {
      blocks.push({ kind: "rule" });
      index += 1;
      continue;
    }
    const heading = HEADING.exec(line);
    if (heading) {
      blocks.push({ kind: "heading", level: heading[1].length, text: heading[2] });
      index += 1;
      continue;
    }
    const table = tableAt(lines, index);
    if (table) {
      const size = table.rows.length;
      blocks.push(table);
      index += 2 + size;
      continue;
    }
    const item = ITEM.exec(line);
    if (item) {
      const ordered = item[1] === undefined;
      const items: string[] = [];
      while (index < lines.length) {
        const next = ITEM.exec(lines[index]);
        if (!next || (next[1] === undefined) !== ordered) break;
        items.push(next[3]);
        index += 1;
      }
      blocks.push({ kind: "list", ordered, items });
      continue;
    }
    if (QUOTE.test(line)) {
      const quoted: string[] = [];
      while (index < lines.length) {
        const next = QUOTE.exec(lines[index]);
        if (!next) break;
        quoted.push(next[1]);
        index += 1;
      }
      blocks.push({ kind: "quote", text: quoted.join("\n") });
      continue;
    }
    const paragraph: string[] = [];
    while (index < lines.length) {
      const next = lines[index];
      if (
        !next.trim() ||
        FENCE.test(next) ||
        HEADING.test(next) ||
        ITEM.test(next) ||
        QUOTE.test(next) ||
        RULE.test(next) ||
        (next.includes("|") && TABLE_RULE.test(lines[index + 1] ?? ""))
      ) {
        break;
      }
      paragraph.push(next);
      index += 1;
    }
    blocks.push({ kind: "paragraph", text: paragraph.join("\n") });
  }
  return blocks;
}

function safeHref(value: string): string | null {
  const href = value.trim();
  return /^https?:\/\/\S+$/i.test(href) ? href : null;
}

export function parseInline(source: string): Inline[] {
  const tokens: Inline[] = [];
  let text = "";
  let index = 0;
  const flush = () => {
    if (text) tokens.push({ kind: "text", text });
    text = "";
  };
  while (index < source.length) {
    if (source[index] === "`") {
      const end = source.indexOf("`", index + 1);
      if (end > index) {
        flush();
        tokens.push({ kind: "code", text: source.slice(index + 1, end) });
        index = end + 1;
        continue;
      }
    }
    const strong = source.startsWith("**", index) ? "**" : source.startsWith("__", index) ? "__" : "";
    if (strong) {
      const end = source.indexOf(strong, index + 2);
      if (end > index + 2) {
        flush();
        tokens.push({ kind: "strong", text: source.slice(index + 2, end) });
        index = end + 2;
        continue;
      }
    }
    if (source[index] === "[") {
      const close = source.indexOf("](", index + 1);
      const end = close < 0 ? -1 : source.indexOf(")", close + 2);
      if (close > index + 1 && end > close + 2) {
        const href = safeHref(source.slice(close + 2, end));
        if (href) {
          flush();
          tokens.push({ kind: "link", text: source.slice(index + 1, close), href });
          index = end + 1;
          continue;
        }
      }
    }
    const marker = source[index];
    if (marker === "*" || marker === "_") {
      const end = source.indexOf(marker, index + 1);
      const intraword =
        marker === "_" &&
        (/[A-Za-z0-9]/.test(source[index - 1] ?? "") ||
          /[A-Za-z0-9]/.test(source[end + 1] ?? ""));
      if (
        end > index + 1 &&
        !intraword &&
        !/\s/.test(source[index + 1]) &&
        !/\s/.test(source[end - 1])
      ) {
        flush();
        tokens.push({ kind: "em", text: source.slice(index + 1, end) });
        index = end + 1;
        continue;
      }
    }
    text += marker;
    index += 1;
  }
  flush();
  return tokens;
}
