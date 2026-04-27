import { useCallback, useLayoutEffect, useRef } from "react";
import { logScrollRestore, snapshotScrollNode } from "./documentScrollRestoreDebug";
import {
  advanceScrollRestore,
  beginScrollRestore,
  type ScrollRestoreProgress
} from "./documentScrollRestoreShared";

export function useDocumentScrollRestore() {
  const documentScrollRef = useRef<HTMLDivElement | null>(null);
  const pendingRestoreRef = useRef<ScrollRestoreProgress | null>(null);
  const frameRef = useRef<number | null>(null);
  const requestIdRef = useRef(0);

  const cancelScheduledRestore = useCallback(() => {
    if (frameRef.current == null) return;
    logScrollRestore("cancel-frame", { frameId: frameRef.current });
    window.cancelAnimationFrame(frameRef.current);
    frameRef.current = null;
  }, []);

  const runPendingRestore = useCallback(() => {
    const node = documentScrollRef.current;
    const pending = pendingRestoreRef.current;
    if (!pending) return;
    if (!node) {
      logScrollRestore("apply-skipped-missing-node", {
        requestId: requestIdRef.current,
        pending
      });
      return;
    }

    logScrollRestore("apply-start", {
      requestId: requestIdRef.current,
      pending,
      node: snapshotScrollNode(node)
    });
    node.scrollTop = pending.targetScrollTop;
    const progressed = advanceScrollRestore(pending, node.scrollTop);
    pendingRestoreRef.current = progressed.done ? null : progressed.next;
    logScrollRestore("apply-finish", {
      requestId: requestIdRef.current,
      progressed,
      node: snapshotScrollNode(node)
    });
    if (progressed.done) return;

    frameRef.current = window.requestAnimationFrame(() => {
      frameRef.current = null;
      runPendingRestore();
    });
  }, []);

  const ensurePendingRestore = useCallback(() => {
    if (!pendingRestoreRef.current || frameRef.current != null) return;
    runPendingRestore();
  }, [runPendingRestore]);

  const captureDocumentScrollPosition = useCallback(() => {
    const node = documentScrollRef.current;
    const scrollTop = node ? node.scrollTop : null;
    logScrollRestore("capture", {
      requestId: requestIdRef.current,
      capturedScrollTop: scrollTop,
      node: snapshotScrollNode(node)
    });
    return scrollTop;
  }, []);

  const restoreDocumentScrollPosition = useCallback((scrollTop: number | null) => {
    cancelScheduledRestore();
    requestIdRef.current += 1;
    pendingRestoreRef.current = scrollTop == null ? null : beginScrollRestore(scrollTop);
    logScrollRestore("request-restore", {
      requestId: requestIdRef.current,
      requestedScrollTop: scrollTop,
      pending: pendingRestoreRef.current,
      node: snapshotScrollNode(documentScrollRef.current)
    });
    ensurePendingRestore();
  }, [cancelScheduledRestore, ensurePendingRestore]);

  useLayoutEffect(() => {
    logScrollRestore("layout-effect", {
      requestId: requestIdRef.current,
      pending: pendingRestoreRef.current,
      node: snapshotScrollNode(documentScrollRef.current)
    });
    ensurePendingRestore();
  }, [ensurePendingRestore]);

  useLayoutEffect(() => {
    return () => cancelScheduledRestore();
  }, [cancelScheduledRestore]);

  return {
    documentScrollRef,
    captureDocumentScrollPosition,
    restoreDocumentScrollPosition
  } as const;
}
