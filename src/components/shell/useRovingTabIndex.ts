import { useCallback, useRef } from "react";

/**
 * Within a group of items (e.g. the sidebar nav), Tab reaches the group once; ArrowUp/
 * ArrowDown/Home/End then move focus between items, matching the standard
 * roving-tabindex pattern for listbox-like widgets.
 */
export function useRovingTabIndex<T extends HTMLElement = HTMLElement>(itemCount: number) {
  const containerRef = useRef<T | null>(null);

  const focusItemAt = useCallback((index: number) => {
    const container = containerRef.current;
    if (!container) return;
    const items = container.querySelectorAll<HTMLElement>('[data-roving-item="true"]');
    const clamped = Math.max(0, Math.min(index, items.length - 1));
    items[clamped]?.focus();
  }, []);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent) => {
      const container = containerRef.current;
      if (!container) return;
      const items = Array.from(
        container.querySelectorAll<HTMLElement>('[data-roving-item="true"]')
      );
      const currentIndex = items.indexOf(document.activeElement as HTMLElement);

      switch (event.key) {
        case "ArrowDown":
          event.preventDefault();
          focusItemAt(currentIndex < 0 ? 0 : (currentIndex + 1) % itemCount);
          break;
        case "ArrowUp":
          event.preventDefault();
          focusItemAt(
            currentIndex < 0 ? itemCount - 1 : (currentIndex - 1 + itemCount) % itemCount
          );
          break;
        case "Home":
          event.preventDefault();
          focusItemAt(0);
          break;
        case "End":
          event.preventDefault();
          focusItemAt(itemCount - 1);
          break;
        default:
          break;
      }
    },
    [focusItemAt, itemCount]
  );

  return { containerRef, handleKeyDown };
}
