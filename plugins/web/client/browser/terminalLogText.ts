const MAX_TEXT = 2 * 1024 * 1024;

/** Turn one raw terminal stream into a bounded, scrollable plain transcript. */
export function terminalPlainText(data: Uint8Array): { text: string; truncated: boolean } {
  const decoded = new TextDecoder("utf-8", { fatal: false }).decode(data);
  let text = decoded
    .replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g, "")
    .replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, "")
    .replace(/\x1b[()][0-2A-Za-z]/g, "")
    .replace(/\x1b[@-_]/g, "")
    .replace(/\r\n/g, "\n")
    .replace(/\r/g, "\n")
    .replace(/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/g, "");
  text = text.replace(/\n{4,}/g, "\n\n\n");
  if (text.length <= MAX_TEXT) return { text, truncated: false };
  return { text: text.slice(text.length - MAX_TEXT), truncated: true };
}
