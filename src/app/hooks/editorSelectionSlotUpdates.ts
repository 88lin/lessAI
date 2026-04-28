import type { SlotTextInput } from "../../lib/api";
import {
  applyEditorSlotOverride,
  buildEditorSlotEdits,
  resolveEditorSlotText,
  type EditorSlotOverrides
} from "../../lib/editorSlots";
import { normalizeNewlines } from "../../lib/helpers";
import type { DocumentSession, EditorSlotEdit, SlotUpdate } from "../../lib/types";
import type {
  DocumentEditorSelectionSnapshot,
  DocumentEditorSelectionSnapshotBase,
  EditorSlotSelectionRange,
  MultiSlotSelectionSnapshot,
  SlotSelectionSnapshot
} from "../../stages/workbench/document/documentEditorTypes";
import { resolveSnapshotRangeInText } from "../../stages/workbench/document/editorSelectionShared";

type SelectionSlotUpdateResult =
  | {
      ok: true;
      slotUpdates: Map<string, string>;
      selectionRanges: EditorSlotSelectionRange[];
      slotEdits: EditorSlotEdit[];
    }
  | {
      ok: false;
      error: string;
    };

function findEditableSlot(session: DocumentSession, slotId: string) {
  const slot = session.writebackSlots.find((item) => item.id === slotId);
  if (!slot || !slot.editable) return null;
  return slot;
}

function selectedTextForMultiSlot(snapshot: MultiSlotSelectionSnapshot, index: number) {
  return snapshot.slotFullTexts[index].slice(
    snapshot.slotStartOffsets[index],
    snapshot.slotEndOffsets[index]
  );
}

export function buildSelectionSlotInputs(
  snapshot: DocumentEditorSelectionSnapshot
): SlotTextInput[] {
  if (snapshot.kind === "text") return [];

  if (snapshot.kind === "slot") {
    return [{ slotId: snapshot.slotId, text: snapshot.text, separatorAfter: "" }];
  }

  return snapshot.slotIds
    .map((slotId, index) => ({
      slotId,
      text: selectedTextForMultiSlot(snapshot, index),
      separatorAfter: snapshot.slotSeparators[index] ?? ""
    }))
    .filter((input) => input.text.trim().length > 0);
}

function resolveSlotSelectionRange(
  currentText: string,
  snapshot: DocumentEditorSelectionSnapshotBase
) {
  return resolveSnapshotRangeInText(currentText, snapshot);
}

function spliceSelection(
  currentText: string,
  range: { startOffset: number; endOffset: number },
  text: string
) {
  return `${currentText.slice(0, range.startOffset)}${normalizeNewlines(text)}${currentText.slice(
    range.endOffset
  )}`;
}

function buildResolvedResult(
  session: DocumentSession,
  slotOverrides: EditorSlotOverrides,
  slotUpdates: Map<string, string>,
  selectionRanges: EditorSlotSelectionRange[]
): SelectionSlotUpdateResult {
  if (slotUpdates.size === 0) {
    return { ok: false, error: "模型未返回可应用的选区改写结果。" };
  }

  let nextOverrides = { ...slotOverrides };
  for (const [slotId, newText] of slotUpdates) {
    const slot = findEditableSlot(session, slotId);
    if (!slot) {
      return { ok: false, error: `槽位 ${slotId} 不可编辑或已不存在。` };
    }
    nextOverrides = applyEditorSlotOverride(nextOverrides, slot, newText);
  }

  return {
    ok: true,
    slotUpdates,
    selectionRanges,
    slotEdits: buildEditorSlotEdits(session, nextOverrides)
  };
}

function resolveSingleSlotUpdate(
  session: DocumentSession,
  slotOverrides: EditorSlotOverrides,
  snapshot: SlotSelectionSnapshot,
  updatesBySlotId: Map<string, string>
): SelectionSlotUpdateResult {
  const replacement = updatesBySlotId.get(snapshot.slotId);
  if (replacement == null) {
    return { ok: false, error: "模型未返回当前选区的改写结果。" };
  }

  const slot = findEditableSlot(session, snapshot.slotId);
  if (!slot) {
    return { ok: false, error: "当前选区不在可编辑片段内，请重新选中后再试。" };
  }

  const currentText = normalizeNewlines(resolveEditorSlotText(slot, slotOverrides));
  const range = resolveSlotSelectionRange(currentText, snapshot);
  if (!range) {
    return { ok: false, error: "选区已变化或文本已被修改，请重新选中后再试。" };
  }

  return buildResolvedResult(
    session,
    slotOverrides,
    new Map([[snapshot.slotId, spliceSelection(currentText, range, replacement)]]),
    [
      {
        slotId: snapshot.slotId,
        startOffset: range.startOffset,
        endOffset: range.startOffset + normalizeNewlines(replacement).length
      }
    ]
  );
}

function resolveMultiSlotUpdate(
  session: DocumentSession,
  slotOverrides: EditorSlotOverrides,
  snapshot: MultiSlotSelectionSnapshot,
  updatesBySlotId: Map<string, string>
): SelectionSlotUpdateResult {
  const slotUpdates = new Map<string, string>();
  const selectionRanges: EditorSlotSelectionRange[] = [];

  for (let index = 0; index < snapshot.slotIds.length; index++) {
    const slotId = snapshot.slotIds[index];
    const replacement = updatesBySlotId.get(slotId);
    if (replacement == null) continue;

    const slot = findEditableSlot(session, slotId);
    if (!slot) {
      return { ok: false, error: `槽位 ${slotId} 不可编辑或已不存在。` };
    }

    const currentText = normalizeNewlines(resolveEditorSlotText(slot, slotOverrides));
    const selectedText = selectedTextForMultiSlot(snapshot, index);
    const range = resolveSlotSelectionRange(currentText, {
      text: selectedText,
      startOffset: snapshot.slotStartOffsets[index],
      endOffset: snapshot.slotEndOffsets[index]
    });
    if (!range) {
      return { ok: false, error: "选区已变化或文本已被修改，请重新选中后再试。" };
    }

    slotUpdates.set(slotId, spliceSelection(currentText, range, replacement));
    selectionRanges.push({
      slotId,
      startOffset: range.startOffset,
      endOffset: range.startOffset + normalizeNewlines(replacement).length
    });
  }

  return buildResolvedResult(session, slotOverrides, slotUpdates, selectionRanges);
}

export function resolveSelectionSlotUpdates(
  session: DocumentSession,
  slotOverrides: EditorSlotOverrides,
  snapshot: DocumentEditorSelectionSnapshot,
  updates: SlotUpdate[]
): SelectionSlotUpdateResult {
  if (snapshot.kind === "text") {
    return { ok: false, error: "纯文本选区不支持槽位写回。" };
  }

  const updatesBySlotId = new Map(updates.map((update) => [update.slotId, update.text] as const));
  return snapshot.kind === "slot"
    ? resolveSingleSlotUpdate(session, slotOverrides, snapshot, updatesBySlotId)
    : resolveMultiSlotUpdate(session, slotOverrides, snapshot, updatesBySlotId);
}
