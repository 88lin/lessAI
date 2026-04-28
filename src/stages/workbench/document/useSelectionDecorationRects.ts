import { useCallback, useEffect, useRef, useState, type RefObject } from "react";

import {
  buildSelectionDecorationRects,
  type SelectionDecorationRect
} from "./SelectionDecorationOverlay";

export interface SelectionDecorationContext<T extends HTMLElement> {
  root: T | null;
  selection: Selection | null;
  range: Range | null;
}

export function resolveContainedSelectionDecorationRange<T extends HTMLElement>({
  root,
  range
}: SelectionDecorationContext<T>) {
  if (
    !root ||
    !range ||
    range.collapsed ||
    !root.contains(range.startContainer) ||
    !root.contains(range.endContainer)
  ) {
    return null;
  }
  return range;
}

export function useSelectionDecorationRects<T extends HTMLElement>({
  rootRef,
  resolveRange = resolveContainedSelectionDecorationRange
}: {
  rootRef: RefObject<T | null>;
  resolveRange?: (context: SelectionDecorationContext<T>) => Range | null;
}) {
  const [selectionDecorationRects, setSelectionDecorationRects] = useState<
    SelectionDecorationRect[]
  >([]);
  const frameRef = useRef<number | null>(null);

  const cancelScheduledSelectionStateSync = useCallback(() => {
    if (frameRef.current == null) return;
    cancelAnimationFrame(frameRef.current);
    frameRef.current = null;
  }, []);

  const clearSelectionDecoration = useCallback(() => {
    cancelScheduledSelectionStateSync();
    setSelectionDecorationRects([]);
  }, [cancelScheduledSelectionStateSync]);

  const syncSelectionState = useCallback(() => {
    const root = rootRef.current;
    const selection = window.getSelection();
    const range = selection?.rangeCount ? selection.getRangeAt(0) : null;
    const activeRange = resolveRange({ root, selection: selection ?? null, range });
    setSelectionDecorationRects(
      activeRange ? buildSelectionDecorationRects(root, activeRange) : []
    );
  }, [resolveRange, rootRef]);

  const scheduleSelectionStateSync = useCallback(() => {
    cancelScheduledSelectionStateSync();
    frameRef.current = requestAnimationFrame(() => {
      frameRef.current = null;
      syncSelectionState();
    });
  }, [cancelScheduledSelectionStateSync, syncSelectionState]);

  useEffect(() => {
    document.addEventListener("selectionchange", syncSelectionState);
    return () => {
      document.removeEventListener("selectionchange", syncSelectionState);
      cancelScheduledSelectionStateSync();
    };
  }, [cancelScheduledSelectionStateSync, syncSelectionState]);

  return {
    selectionDecorationRects,
    clearSelectionDecoration,
    scheduleSelectionStateSync,
    syncSelectionState
  };
}
