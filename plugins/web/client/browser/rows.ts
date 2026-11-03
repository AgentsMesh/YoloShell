import { dot, el } from "./dom.js";

/** One sidebar row; identity is carried by data attributes for delegation. */
export function row(
  kind: string,
  id: string,
  machine: string,
  title: string,
  context: string,
  mark: string,
  status: { key: string; label: string },
  selected: boolean,
): HTMLLIElement {
  const item = el("li");
  const button = el("button", "row");
  button.type = "button";
  button.dataset.kind = kind;
  button.dataset.id = id;
  button.dataset.machine = machine;
  if (selected) button.classList.add("is-selected");
  const copy = el("span", "row-copy");
  copy.append(el("span", "row-title", title), el("span", "row-context", context));
  const stateLabel = el("span", `row-state state-${status.key}`);
  stateLabel.append(dot(status.key), el("span", "", status.label));
  button.append(el("span", "row-mark", mark), copy, stateLabel);
  item.append(button);
  return item;
}
