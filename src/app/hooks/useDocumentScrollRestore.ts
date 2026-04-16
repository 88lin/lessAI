import { useCallback, useLayoutEffect, useRef } from "react";

export function useDocumentScrollRestore() {
  const documentScrollRef = useRef<HTMLDivElement | null>(null);
  const pendingScrollTopRef = useRef<number | null>(null);

  const captureDocumentScrollPosition = useCallback(() => {
    const node = documentScrollRef.current;
    return node ? node.scrollTop : null;
  }, []);

  const restoreDocumentScrollPosition = useCallback((scrollTop: number | null) => {
    pendingScrollTopRef.current = scrollTop;
  }, []);

  useLayoutEffect(() => {
    const pending = pendingScrollTopRef.current;
    const node = documentScrollRef.current;
    if (pending == null || !node) return;

    node.scrollTop = pending;
    const frame = window.requestAnimationFrame(() => {
      if (pendingScrollTopRef.current === pending) {
        pendingScrollTopRef.current = null;
      }
    });

    return () => window.cancelAnimationFrame(frame);
  });

  return {
    documentScrollRef,
    captureDocumentScrollPosition,
    restoreDocumentScrollPosition
  } as const;
}
