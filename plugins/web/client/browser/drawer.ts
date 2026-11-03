import { byId } from "./dom.js";

export function closeDrawer(): void {
  document.body.classList.remove("sidebar-open");
  byId("menu").setAttribute("aria-expanded", "false");
}
