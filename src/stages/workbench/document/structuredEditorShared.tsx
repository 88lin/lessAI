import { memo, useEffect, useLayoutEffect, useRef } from "react";

import { normalizeNewlines } from "../../../lib/helpers";
import type { WritebackSlot } from "../../../lib/types";

export function slotPresentationClass(
  slot: WritebackSlot,
  options?: { baseClassName?: string; protectedClassName?: string }
) {
  const presentation = slot.presentation;
  const baseClassName = options?.baseClassName ?? "structured-editor-slot";
  const protectedClassName = options?.protectedClassName ?? "is-locked";

  return [
    baseClassName,
    slot.editable ? "is-editable" : protectedClassName,
    presentation?.bold ? "is-bold" : "",
    presentation?.italic ? "is-italic" : "",
    presentation?.underline ? "is-underline" : "",
    presentation?.href ? "is-link" : ""
  ]
    .filter(Boolean)
    .join(" ");
}

export const EditableSlotSpan = memo(function EditableSlotSpan({
  slot,
  text,
  registerNode,
  classNameOptions
}: {
  slot: WritebackSlot;
  text: string;
  registerNode: (slotId: string, node: HTMLSpanElement | null) => void;
  classNameOptions?: { baseClassName?: string; protectedClassName?: string };
}) {
  const nodeRef = useRef<HTMLSpanElement | null>(null);
  const lastSyncedTextRef = useRef<string | null>(null);

  useEffect(() => {
    registerNode(slot.id, nodeRef.current);
    return () => {
      registerNode(slot.id, null);
      lastSyncedTextRef.current = null;
    };
  }, [registerNode, slot.id]);

  useLayoutEffect(() => {
    const node = nodeRef.current;
    if (!node) return;

    // On first mount, initialize text directly without forcing a layout read.
    if (lastSyncedTextRef.current == null) {
      node.textContent = text;
      lastSyncedTextRef.current = text;
      return;
    }

    if (lastSyncedTextRef.current === text) return;
    if (normalizeNewlines(node.innerText) === text) {
      lastSyncedTextRef.current = text;
      return;
    }
    if (document.activeElement === node) {
      lastSyncedTextRef.current = normalizeNewlines(node.innerText);
      return;
    }

    node.textContent = text;
    lastSyncedTextRef.current = text;
  }, [text]);

  return (
    <span
      ref={nodeRef}
      className={slotPresentationClass(slot, classNameOptions)}
      aria-label={`编辑槽位 ${slot.order + 1}`}
      data-slot-id={slot.id}
    />
  );
});
