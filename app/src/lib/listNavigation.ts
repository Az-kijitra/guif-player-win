// Keyboard movement shared by the lists and the album grid.
// Each list keeps one item in the Tab order (tabindex 0) and moves focus with the arrow keys.

const NAV_ITEM_SELECTOR = "[data-nav-item]";

export function nextListIndex(
  key: string,
  current: number,
  count: number,
  { columns = 1, pageSize = 10 }: { columns?: number; pageSize?: number } = {}
): number | null {
  if (count <= 0) return null;
  const last = count - 1;
  switch (key) {
    case "ArrowDown":
      return Math.min(current + columns, last);
    case "ArrowUp":
      return Math.max(current - columns, 0);
    case "ArrowRight":
      return columns > 1 ? Math.min(current + 1, last) : null;
    case "ArrowLeft":
      return columns > 1 ? Math.max(current - 1, 0) : null;
    case "PageDown":
      return Math.min(current + pageSize, last);
    case "PageUp":
      return Math.max(current - pageSize, 0);
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return null;
  }
}

export function hasCommandModifier(e: KeyboardEvent): boolean {
  return e.ctrlKey || e.altKey || e.metaKey;
}

export function focusNavItem(container: HTMLElement | null, index: number) {
  const item = container?.querySelectorAll<HTMLElement>(NAV_ITEM_SELECTOR)[index];
  item?.focus();
  item?.scrollIntoView({ block: "nearest" });
}

/** Moves focus back to the list on screen. Returns false when there is none. */
export function focusActiveNavItem(): boolean {
  const item = document.querySelector<HTMLElement>(`${NAV_ITEM_SELECTOR}[tabindex="0"]`);
  item?.focus();
  return item !== null;
}

/** How many items fit between the top and the bottom of the visible area. */
export function visibleItemCount(container: HTMLElement | null): number {
  const item = container?.querySelector<HTMLElement>(NAV_ITEM_SELECTOR);
  let scroller = container;
  while (scroller && scroller.clientHeight === scroller.scrollHeight && scroller.parentElement) {
    scroller = scroller.parentElement;
  }
  if (!item || !scroller || item.offsetHeight === 0) return 10;
  return Math.max(1, Math.floor(scroller.clientHeight / item.offsetHeight));
}
