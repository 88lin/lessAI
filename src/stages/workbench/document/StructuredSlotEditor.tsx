import {
  forwardRef,
  memo,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef
} from "react";

import {
  applyEditorSlotOverride,
  buildEditorSlotEdits,
  buildEditorTextFromSession,
  resolveEditorSlotText
} from "../../../lib/editorSlots";
import { normalizeNewlines } from "../../../lib/helpers";
import type {
  DocumentEditorHandle,
  DocumentEditorProps,
  DocumentEditorPreviewResult,
  DocumentEditorSelectionSnapshot,
  SlotSelectionSnapshot
} from "./documentEditorTypes";
import { buildSelectionSnapshotBase, resolveSnapshotRangeInText } from "./editorSelectionShared";
import { StructuredEditorUnit } from "./StructuredEditorUnit";
import { useProgressiveRevealCount } from "../hooks/useProgressiveRevealCount";

function buildSlotSelectionSnapshot(
  node: HTMLElement,
  slotId: string,
  range: Range
): SlotSelectionSnapshot | null {
  const base = buildSelectionSnapshotBase(node, range);
  if (!base) return null;

  return {
    kind: "slot",
    slotId,
    ...base
  };
}

function replaceSelectionText(
  currentText: string,
  snapshot: SlotSelectionSnapshot,
  replacementText: string
) {
  const replacement = normalizeNewlines(replacementText);
  if (replacement.trim().length === 0) {
    return { ok: false, error: "模型返回内容为空，已取消替换。" } as const;
  }

  const resolvedRange = resolveSnapshotRangeInText(currentText, snapshot);
  if (!resolvedRange) {
    return { ok: false, error: "选区已变化或文本已被修改，请重新选中后再试。" } as const;
  }

  return {
    ok: true,
    text: `${currentText.slice(0, resolvedRange.startOffset)}${replacement}${currentText.slice(
      resolvedRange.endOffset
    )}`
  } as const;
}

export const StructuredSlotEditor = memo(
  forwardRef<DocumentEditorHandle, DocumentEditorProps>(function StructuredSlotEditor(
    {
      session,
      slotOverrides,
      showMarkers,
      dirty,
      busy,
      onChange,
      onChangeSlotText,
      onSave,
      onSelectionChange
    },
    ref
  ) {
    const slotNodesRef = useRef<Record<string, HTMLSpanElement | null>>({});
    const editableSlotIdSetRef = useRef<Set<string>>(new Set());
    const nodeSlotIdMapRef = useRef<WeakMap<Node, string>>(new WeakMap());
    const hasSelectionRef = useRef(false);

    const registerNode = useCallback((slotId: string, node: HTMLSpanElement | null) => {
      const previous = slotNodesRef.current[slotId];
      if (previous) {
        nodeSlotIdMapRef.current.delete(previous);
      }
      slotNodesRef.current[slotId] = node;
      if (node) {
        nodeSlotIdMapRef.current.set(node, slotId);
      }
    }, []);

    const findSessionSlot = useCallback(
      (slotId: string) => session.writebackSlots.find((item) => item.id === slotId) ?? null,
      [session.writebackSlots]
    );

    useEffect(() => {
      const set = new Set<string>();
      for (const slot of session.writebackSlots) {
        if (slot.editable) {
          set.add(slot.id);
        }
      }
      editableSlotIdSetRef.current = set;
    }, [session.writebackSlots]);

    const captureSlotSelection = useCallback(() => {
      const selection = window.getSelection();
      const range = selection?.rangeCount ? selection.getRangeAt(0) : null;
      if (!range) return null;

      const resolveSlotIdFromNode = (node: Node | null): string | null => {
        let current: Node | null = node;
        while (current) {
          const mapped = nodeSlotIdMapRef.current.get(current);
          if (mapped && editableSlotIdSetRef.current.has(mapped)) {
            return mapped;
          }
          if (current instanceof HTMLElement) {
            const direct = current.dataset.slotId;
            if (direct && editableSlotIdSetRef.current.has(direct)) {
              return direct;
            }
          }
          current = current.parentNode;
        }
        return null;
      };

      const startSlotId = resolveSlotIdFromNode(range.startContainer);
      if (!startSlotId) return null;
      const endSlotId = resolveSlotIdFromNode(range.endContainer);
      if (!endSlotId || endSlotId !== startSlotId) return null;

      const node = slotNodesRef.current[startSlotId];
      if (!node) return null;
      if (!node.contains(range.startContainer) || !node.contains(range.endContainer)) {
        return null;
      }
      return buildSlotSelectionSnapshot(node, startSlotId, range);
    }, [session.writebackSlots]);

    useEffect(() => {
      const handleKeyDown = (event: KeyboardEvent) => {
        const key = event.key.toLowerCase();
        if (!(event.ctrlKey || event.metaKey) || key !== "s") return;
        event.preventDefault();
        if (!dirty || busy) return;
        onSave();
      };
      window.addEventListener("keydown", handleKeyDown);
      return () => window.removeEventListener("keydown", handleKeyDown);
    }, [busy, dirty, onSave]);

    useEffect(() => {
      const firstEditable = session.writebackSlots.find((slot) => slot.editable);
      if (!firstEditable) return;
      const id = requestAnimationFrame(() => {
        slotNodesRef.current[firstEditable.id]?.focus();
      });
      return () => cancelAnimationFrame(id);
    }, [session.id, session.writebackSlots]);

    const renderedUnitCount = useProgressiveRevealCount({
      total: session.rewriteUnits.length,
      key: session.id,
      enabled: session.rewriteUnits.length > 120,
      initial: 80,
      step: 120
    });

    useEffect(() => {
      if (!onSelectionChange) return;

      const handleSelectionChange = () => {
        const next = captureSlotSelection() != null;
        if (next === hasSelectionRef.current) return;
        hasSelectionRef.current = next;
        onSelectionChange(next);
      };

      document.addEventListener("selectionchange", handleSelectionChange);
      return () => document.removeEventListener("selectionchange", handleSelectionChange);
    }, [captureSlotSelection, onSelectionChange]);

    const resolveSelectionReplacement = useCallback(
      (
        snapshot: DocumentEditorSelectionSnapshot,
        replacementText: string
      ):
        | {
            ok: true;
            slotId: string;
            replacedText: string;
            value: string;
            slotEdits: ReturnType<typeof buildEditorSlotEdits>;
          }
        | {
            ok: false;
            error: string;
          } => {
        if (snapshot.kind !== "slot") {
          return { ok: false, error: "请在单个可编辑片段内重新选中后再试。" };
        }

        const slot = findSessionSlot(snapshot.slotId);
        if (!slot || !slot.editable) {
          return { ok: false, error: "当前选区不在可编辑片段内，请重新选中后再试。" };
        }

        const currentText = normalizeNewlines(
          slotNodesRef.current[slot.id]?.innerText ?? resolveEditorSlotText(slot, slotOverrides)
        );
        const replaced = replaceSelectionText(currentText, snapshot, replacementText);
        if (!replaced.ok) return replaced;

        const nextOverrides = applyEditorSlotOverride(slotOverrides, slot, replaced.text);
        return {
          ok: true,
          slotId: slot.id,
          replacedText: replaced.text,
          value: buildEditorTextFromSession(session, nextOverrides),
          slotEdits: buildEditorSlotEdits(session, nextOverrides)
        };
      },
      [findSessionSlot, session, slotOverrides]
    );

    const previewSelectionReplacement = useCallback(
      (
        snapshot: DocumentEditorSelectionSnapshot,
        replacementText: string
      ): DocumentEditorPreviewResult => {
        const resolved = resolveSelectionReplacement(snapshot, replacementText);
        if (!resolved.ok) return resolved;
        return {
          ok: true,
          value: resolved.value,
          slotEdits: resolved.slotEdits
        };
      },
      [resolveSelectionReplacement]
    );

    useImperativeHandle(
      ref,
      (): DocumentEditorHandle => ({
        captureSelection: captureSlotSelection,
        previewSelectionReplacement,
        applySelectionReplacement: (snapshot, replacementText) => {
          const resolved = resolveSelectionReplacement(snapshot, replacementText);
          if (!resolved.ok) return resolved;

          const node = slotNodesRef.current[resolved.slotId];
          if (node) {
            node.innerText = resolved.replacedText;
            node.focus();
          }
          onChangeSlotText(resolved.slotId, resolved.replacedText);
          onChange(resolved.value);
          return { ok: true };
        },
        collectSlotEdits: () => buildEditorSlotEdits(session, slotOverrides)
      }),
      [
        captureSlotSelection,
        onChange,
        onChangeSlotText,
        previewSelectionReplacement,
        resolveSelectionReplacement,
        session,
        slotOverrides
      ]
    );

    const visibleRewriteUnitIdSet = useMemo(
      () => new Set(session.rewriteUnits.slice(0, renderedUnitCount).map((item) => item.id)),
      [renderedUnitCount, session.rewriteUnits]
    );

    const renderedUnits = session.rewriteUnits.map((rewriteUnit) => {
      if (!visibleRewriteUnitIdSet.has(rewriteUnit.id)) {
        return null;
      }
      return (
        <StructuredEditorUnit
          key={rewriteUnit.id}
          session={session}
          rewriteUnit={rewriteUnit}
          slotOverrides={slotOverrides}
          busy={busy}
          registerNode={registerNode}
          onChangeSlotText={onChangeSlotText}
        />
      );
    });

    return (
      <div
        className={`document-flow-wrap structured-editor-wrap ${showMarkers ? "is-markers" : "is-quiet"}`}
      >
        {showMarkers ? (
          <div className="unit-legend" aria-label="高亮说明">
            <span className="legend-chip is-editable" title="可改写单元（包含可编辑槽位）">
              可改写
            </span>
            <span className="legend-chip is-protected" title="保护区（锁定内容，保持只读）">
              保护区
            </span>
          </div>
        ) : null}

        <div className="workbench-editor-editable structured-editor-flow" aria-label="编辑终稿">
          {renderedUnits}
          {renderedUnitCount < session.rewriteUnits.length ? (
            <span className="doc-unit-wrap" aria-hidden="true" />
          ) : null}
        </div>
      </div>
    );
  })
);
