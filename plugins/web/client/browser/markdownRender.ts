import { parseInline, parseMarkdown } from "./markdown.js";
import type { Block } from "./markdownTypes.js";

function inline(parent: ParentNode, source: string): void {
  for (const token of parseInline(source)) {
    if (token.kind === "text") {
      const lines = token.text.split("\n");
      lines.forEach((line, at) => {
        if (at > 0) parent.appendChild(document.createElement("br"));
        if (line) parent.appendChild(document.createTextNode(line));
      });
    } else if (token.kind === "link") {
      const anchor = document.createElement("a");
      anchor.textContent = token.text;
      anchor.href = token.href;
      anchor.target = "_blank";
      anchor.rel = "noopener noreferrer";
      parent.appendChild(anchor);
    } else {
      const element = document.createElement(token.kind === "code" ? "code" : token.kind);
      element.textContent = token.text;
      parent.appendChild(element);
    }
  }
}

function block(item: Block): ParentNode {
  if (item.kind === "code") {
    const pre = document.createElement("pre");
    const code = document.createElement("code");
    code.textContent = item.text;
    if (item.language) code.dataset.language = item.language;
    pre.appendChild(code);
    return pre;
  }
  if (item.kind === "table") {
    const table = document.createElement("table");
    const head = document.createElement("thead");
    const headRow = document.createElement("tr");
    for (const value of item.head) {
      const cell = document.createElement("th");
      inline(cell, value);
      headRow.appendChild(cell);
    }
    head.appendChild(headRow);
    table.appendChild(head);
    const body = document.createElement("tbody");
    for (const values of item.rows) {
      const row = document.createElement("tr");
      for (const value of values) {
        const cell = document.createElement("td");
        inline(cell, value);
        row.appendChild(cell);
      }
      body.appendChild(row);
    }
    table.appendChild(body);
    return table;
  }
  if (item.kind === "rule") return document.createElement("hr");
  if (item.kind === "list") {
    const list = document.createElement(item.ordered ? "ol" : "ul");
    for (const value of item.items) {
      const entry = document.createElement("li");
      inline(entry, value);
      list.appendChild(entry);
    }
    return list;
  }
  if (item.kind === "heading") {
    const heading = document.createElement(`h${Math.min(item.level + 1, 5)}`);
    inline(heading, item.text);
    return heading;
  }
  if (item.kind === "quote") {
    const quote = document.createElement("blockquote");
    inline(quote, item.text);
    return quote;
  }
  const paragraph = document.createElement("p");
  inline(paragraph, item.text);
  return paragraph;
}

/** Safe, dependency-free Markdown rendering: the source is never parsed as HTML. */
export function renderMarkdown(source: string): DocumentFragment {
  const fragment = document.createDocumentFragment();
  for (const item of parseMarkdown(source)) fragment.appendChild(block(item));
  return fragment;
}
