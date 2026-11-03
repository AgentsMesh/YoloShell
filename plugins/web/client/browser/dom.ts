export function byId<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

export function el<K extends keyof HTMLElementTagNameMap>(
  tag: K,
  className = "",
  text = "",
): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  if (className) element.className = className;
  if (text) element.textContent = text;
  return element;
}

export function dot(key: string): HTMLSpanElement {
  return el("span", `state-dot state-${key}`);
}
