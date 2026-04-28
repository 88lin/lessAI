import {
  forwardRef,
  memo,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef
} from "react";
import type { ClipboardEvent } from "react";

import { normalizeNewlines } from "../../../lib/helpers";
import type {
  DocumentEditorHandle,
  DocumentEditorProps,
  DocumentEditorSelectionSnapshot,
  DocumentEditorPreviewResult,
} from "./documentEditorTypes";
import { buildSelectionSnapshotBase, resolveSnapshotRangeInText } from "./editorSelectionShared";
import { SelectionDecorationOverlay } from "./SelectionDecorationOverlay";
import { useEditorSaveShortcut } from "./useEditorSaveShortcut";
import {
  resolveContainedSelectionDecorationRange,
  type SelectionDecorationContext,
  useSelectionDecorationRects
} from "./useSelectionDecorationRects";

function buildSelectionSnapshot(
  node: HTMLDivElement,
  range: Range
): DocumentEditorSelectionSnapshot | null {
  const base = buildSelectionSnapshotBase(node, range);
  if (!base) return null;

  return {
    kind: "text",
    ...base
  };
}

function previewReplacementValue(
  node: HTMLDivElement,
  snapshot: DocumentEditorSelectionSnapshot,
  replacementText: string
): DocumentEditorPreviewResult {
  if (snapshot.kind !== "text") {
    return { ok: false, error: "当前选区类型与编辑器不匹配，请重新选中后再试。" };
  }

  const replacement = normalizeNewlines(replacementText);
  if (replacement.trim().length === 0) {
    return { ok: false, error: "模型返回内容为空，已取消替换。" };
  }

  const currentValue = normalizeNewlines(node.innerText);
  const resolvedRange = resolveSnapshotRangeInText(currentValue, snapshot);
  if (!resolvedRange) {
    return { ok: false, error: "选区已变化或文本已被修改，请重新选中后再试。" };
  }

  return {
    ok: true,
    value: `${currentValue.slice(0, resolvedRange.startOffset)}${replacement}${currentValue.slice(
      resolvedRange.endOffset
    )}`
  };
}

export const PlainTextDocumentEditor = memo(
  forwardRef<DocumentEditorHandle, DocumentEditorProps>(function PlainTextDocumentEditor(
    { value, dirty, busy, onChange, onSave, onSelectionChange }: DocumentEditorProps,
    ref
  ) {
    const editorFieldRef = useRef<HTMLDivElement | null>(null);
    const hasSelectionRef = useRef(false);
    const lastSyncedTextRef = useRef<string | null>(null);

    useEditorSaveShortcut({ busy, dirty, onSave });

    const setEditorSelectionAvailable = useCallback(
      (next: boolean) => {
        if (next === hasSelectionRef.current) return;
        hasSelectionRef.current = next;
        onSelectionChange?.(next);
      },
      [onSelectionChange]
    );

    const resolveSelectionDecorationRange = useCallback(
      (context: SelectionDecorationContext<HTMLDivElement>) => {
        const range = resolveContainedSelectionDecorationRange(context);
        setEditorSelectionAvailable(range != null);
        return range;
      },
      [setEditorSelectionAvailable]
    );

    const {
      selectionDecorationRects,
      clearSelectionDecoration,
      scheduleSelectionStateSync
    } = useSelectionDecorationRects({
      rootRef: editorFieldRef,
      resolveRange: resolveSelectionDecorationRange
    });

    const clearSelectionState = useCallback(() => {
      clearSelectionDecoration();
      setEditorSelectionAvailable(false);
    }, [clearSelectionDecoration, setEditorSelectionAvailable]);

    useEffect(() => {
      const node = editorFieldRef.current;
      if (!node) return;

      if (lastSyncedTextRef.current == null) {
        node.textContent = value;
        lastSyncedTextRef.current = value;
        return;
      }

      if (lastSyncedTextRef.current === value) return;
      if (document.activeElement === node && dirty) {
        lastSyncedTextRef.current = normalizeNewlines(node.innerText);
        return;
      }

      node.textContent = value;
      lastSyncedTextRef.current = value;
    }, [dirty, value]);

    useEffect(() => {
      const id = requestAnimationFrame(() => {
        editorFieldRef.current?.focus();
      });
      return () => cancelAnimationFrame(id);
    }, []);

    useImperativeHandle(
      ref,
      (): DocumentEditorHandle => ({
        captureSelection: () => {
          const node = editorFieldRef.current;
          const selection = window.getSelection();
          if (!node || !selection?.rangeCount) return null;
          return buildSelectionSnapshot(node, selection.getRangeAt(0));
        },

        previewSelectionReplacement: (snapshot, replacementText) => {
          const node = editorFieldRef.current;
          if (!node) return { ok: false, error: "编辑器尚未就绪。" };
          return previewReplacementValue(node, snapshot, replacementText);
        },

        applySelectionReplacement: (snapshot, replacementText) => {
          const node = editorFieldRef.current;
          if (!node) return { ok: false, error: "编辑器尚未就绪。" };

          const preview = previewReplacementValue(node, snapshot, replacementText);
          if (!preview.ok) return preview;

          node.textContent = preview.value;
          lastSyncedTextRef.current = preview.value;
          node.focus();
          clearSelectionState();
          onChange(preview.value);
          return { ok: true };
        },

        collectSlotEdits: () => null,

        applySlotUpdates: (_slotUpdates: Map<string, string>, _options) => {
          // 纯文本编辑器无槽位概念，不需要逐槽位应用。
        }
      }),
      [clearSelectionState, onChange]
    );

    const handleEditorInput = useCallback(() => {
      const node = editorFieldRef.current;
      if (!node) return;
      const nextText = normalizeNewlines(node.innerText);
      lastSyncedTextRef.current = nextText;
      onChange(nextText);
      scheduleSelectionStateSync();
    }, [onChange, scheduleSelectionStateSync]);

    const handleEditorPaste = useCallback((event: ClipboardEvent<HTMLDivElement>) => {
      event.preventDefault();
      const text = event.clipboardData.getData("text/plain");
      if (!text) return;

      if (document.execCommand("insertText", false, text)) {
        scheduleSelectionStateSync();
        return;
      }
      const selection = window.getSelection();
      if (!selection?.rangeCount) return;
      selection.deleteFromDocument();
      selection.getRangeAt(0).insertNode(document.createTextNode(text));
      selection.collapseToEnd();
      const node = editorFieldRef.current;
      if (node) {
        const nextText = normalizeNewlines(node.innerText);
        lastSyncedTextRef.current = nextText;
        onChange(nextText);
        scheduleSelectionStateSync();
      }
    }, [onChange, scheduleSelectionStateSync]);

    return (
      <div className="workbench-editor-selection-shell">
        <div
          ref={editorFieldRef}
          className={`document-flow workbench-editor-editable ${
            value.trim().length === 0 ? "is-empty" : ""
          }`}
          contentEditable={!busy}
          role="textbox"
          aria-multiline="true"
          aria-label="编辑终稿"
          tabIndex={0}
          spellCheck={false}
          data-placeholder="在此编辑终稿…"
          onPointerDown={clearSelectionState}
          onPointerUp={scheduleSelectionStateSync}
          onSelect={scheduleSelectionStateSync}
          onKeyUp={scheduleSelectionStateSync}
          onInput={handleEditorInput}
          onPaste={handleEditorPaste}
          suppressContentEditableWarning
        />
        <SelectionDecorationOverlay rects={selectionDecorationRects} />
      </div>
    );
  })
);
