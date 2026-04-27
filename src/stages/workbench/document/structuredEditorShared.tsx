import { memo, useCallback, useEffect, useLayoutEffect, useRef } from "react";
import type { ClipboardEvent } from "react";

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
  busy,
  registerNode,
  onChange,
  classNameOptions
}: {
  slot: WritebackSlot;
  text: string;
  busy: boolean;
  registerNode: (slotId: string, node: HTMLSpanElement | null) => void;
  onChange: (slotId: string, value: string) => void;
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
    if (document.activeElement === node) {
      lastSyncedTextRef.current = normalizeNewlines(node.innerText);
      return;
    }

    node.textContent = text;
    lastSyncedTextRef.current = text;
  }, [text]);

  const handleInput = useCallback(() => {
    const node = nodeRef.current;
    if (!node) return;
    const nextText = normalizeNewlines(node.innerText);
    lastSyncedTextRef.current = nextText;
    onChange(slot.id, nextText);
  }, [onChange, slot.id]);

  const handlePaste = useCallback((event: ClipboardEvent<HTMLSpanElement>) => {
    event.preventDefault();
    const text = event.clipboardData.getData("text/plain");
    if (!text) return;

    if (document.execCommand("insertText", false, text)) return;
    const selection = window.getSelection();
    if (!selection?.rangeCount) return;
    selection.deleteFromDocument();
    selection.getRangeAt(0).insertNode(document.createTextNode(text));
    selection.collapseToEnd();
  }, []);

  return (
    <span
      ref={nodeRef}
      className={slotPresentationClass(slot, classNameOptions)}
      contentEditable={!busy}
      suppressContentEditableWarning
      spellCheck={false}
      role="textbox"
      aria-label={`编辑槽位 ${slot.order + 1}`}
      data-slot-id={slot.id}
      onInput={handleInput}
      onPaste={handlePaste}
    />
  );
});
