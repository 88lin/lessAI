import { forwardRef, memo, useCallback, useEffect, useImperativeHandle, useRef } from "react";
import type { ClipboardEvent } from "react";
import { normalizeNewlines } from "../../../lib/helpers";

export interface DocumentEditorSelectionSnapshot {
  text: string;
  range: Range;
}

export type DocumentEditorApplyResult =
  | { ok: true }
  | { ok: false; error: string };

export interface DocumentEditorHandle {
  captureSelection: () => DocumentEditorSelectionSnapshot | null;
  applySelectionReplacement: (
    snapshot: DocumentEditorSelectionSnapshot,
    replacementText: string
  ) => DocumentEditorApplyResult;
}

interface DocumentEditorProps {
  value: string;
  dirty: boolean;
  busy: boolean;
  onChange: (value: string) => void;
  onSave: () => void;
  onSelectionChange?: (hasSelection: boolean) => void;
}

export const DocumentEditor = memo(
  forwardRef<DocumentEditorHandle, DocumentEditorProps>(function DocumentEditor(
    { value, dirty, busy, onChange, onSave, onSelectionChange }: DocumentEditorProps,
    ref
  ) {
  const editorFieldRef = useRef<HTMLDivElement | null>(null);
  const hasSelectionRef = useRef<boolean>(false);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      const saveCombo = (event.ctrlKey || event.metaKey) && key === "s";
      if (!saveCombo) return;

      event.preventDefault();
      if (!dirty || busy) return;
      onSave();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [busy, dirty, onSave]);

  useEffect(() => {
    const node = editorFieldRef.current;
    if (!node) return;

    const domText = normalizeNewlines(node.innerText);
    if (domText === value) return;
    if (document.activeElement === node && dirty) return;

    node.innerText = value;
  }, [dirty, value]);

  useEffect(() => {
    const node = editorFieldRef.current;
    if (!node) return;

    requestAnimationFrame(() => {
      node.focus();
    });
  }, []);

  useEffect(() => {
    if (!onSelectionChange) return;

    const handleSelectionChange = () => {
      const node = editorFieldRef.current;
      if (!node) return;

      const selection = window.getSelection();
      if (!selection || selection.rangeCount === 0) {
        if (hasSelectionRef.current) {
          hasSelectionRef.current = false;
          onSelectionChange(false);
        }
        return;
      }

      const range = selection.getRangeAt(0);
      const withinEditor =
        node.contains(range.startContainer) && node.contains(range.endContainer);
      const nextHasSelection = withinEditor && !range.collapsed;
      if (nextHasSelection === hasSelectionRef.current) return;
      hasSelectionRef.current = nextHasSelection;
      onSelectionChange(nextHasSelection);
    };

    document.addEventListener("selectionchange", handleSelectionChange);
    return () => {
      document.removeEventListener("selectionchange", handleSelectionChange);
    };
  }, [onSelectionChange]);

  useImperativeHandle(
    ref,
    (): DocumentEditorHandle => ({
      captureSelection: () => {
        const node = editorFieldRef.current;
        if (!node) return null;

        const selection = window.getSelection();
        if (!selection || selection.rangeCount === 0) return null;

        const range = selection.getRangeAt(0);
        if (range.collapsed) return null;
        if (!node.contains(range.startContainer) || !node.contains(range.endContainer)) {
          return null;
        }

        const text = normalizeNewlines(range.toString());
        if (text.trim().length === 0) return null;

        return { text, range: range.cloneRange() };
      },

      applySelectionReplacement: (snapshot, replacementText) => {
        const node = editorFieldRef.current;
        if (!node) return { ok: false, error: "编辑器尚未就绪。" };

        const replacement = normalizeNewlines(replacementText);
        if (replacement.trim().length === 0) {
          return { ok: false, error: "模型返回内容为空，已取消替换。" };
        }

        const currentSelected = normalizeNewlines(snapshot.range.toString());
        if (currentSelected !== snapshot.text) {
          return {
            ok: false,
            error: "选区已变化或文本已被修改，请重新选中后再试。"
          };
        }

        const selection = window.getSelection();
        if (!selection) return { ok: false, error: "无法读取当前选区。" };

        selection.removeAllRanges();
        selection.addRange(snapshot.range);

        const ok = document.execCommand("insertText", false, replacement);
        if (!ok) {
          selection.deleteFromDocument();
          if (selection.rangeCount === 0) {
            return { ok: false, error: "替换失败：选区范围不可用。" };
          }
          selection
            .getRangeAt(0)
            .insertNode(document.createTextNode(replacement));
          selection.collapseToEnd();
        }

        onChange(normalizeNewlines(node.innerText));
        return { ok: true };
      }
    }),
    [onChange]
  );

  const handleEditorInput = useCallback(() => {
    const node = editorFieldRef.current;
    if (!node) return;
    onChange(normalizeNewlines(node.innerText));
  }, [onChange]);

  const handleEditorPaste = useCallback((event: ClipboardEvent<HTMLDivElement>) => {
    event.preventDefault();
    const text = event.clipboardData.getData("text/plain");
    if (!text) return;

    const ok = document.execCommand("insertText", false, text);
    if (ok) return;

    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return;
    selection.deleteFromDocument();
    selection.getRangeAt(0).insertNode(document.createTextNode(text));
    selection.collapseToEnd();
  }, []);

  return (
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
      onInput={handleEditorInput}
      onPaste={handleEditorPaste}
      suppressContentEditableWarning
    />
  );
  })
);
