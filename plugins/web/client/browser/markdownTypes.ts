export type Block =
  | { kind: "paragraph"; text: string }
  | { kind: "heading"; level: number; text: string }
  | { kind: "code"; language?: string; text: string }
  | { kind: "list"; ordered: boolean; items: string[] }
  | { kind: "quote"; text: string }
  | { kind: "table"; head: string[]; rows: string[][] }
  | { kind: "rule" };

export type Inline =
  | { kind: "text"; text: string }
  | { kind: "code"; text: string }
  | { kind: "strong"; text: string }
  | { kind: "em"; text: string }
  | { kind: "link"; text: string; href: string };
