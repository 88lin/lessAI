import type { EditorSlotOverrides } from "../../../lib/editorSlots";
import type { DocumentSession, EditorSlotEdit } from "../../../lib/types";

export interface DocumentEditorSelectionSnapshotBase {
  text: string;
  startOffset: number;
  endOffset: number;
}

export interface PlainTextSelectionSnapshot extends DocumentEditorSelectionSnapshotBase {
  kind: "text";
}

export interface SlotSelectionSnapshot extends DocumentEditorSelectionSnapshotBase {
  kind: "slot";
  slotId: string;
}

export interface MultiSlotSelectionSnapshot extends DocumentEditorSelectionSnapshotBase {
  kind: "multiSlot";
  /** 按文档顺序排列的涉及槽位 ID */
  slotIds: string[];
  /** 每个槽位当前完整文本（与 slotIds 一一对应） */
  slotFullTexts: string[];
  /** 每个槽位内真实选区起点（与 slotIds 一一对应） */
  slotStartOffsets: number[];
  /** 每个槽位内真实选区终点（与 slotIds 一一对应） */
  slotEndOffsets: number[];
  /** 每个槽位的分隔符（与 slotIds 一一对应） */
  slotSeparators: string[];
}

export type DocumentEditorSelectionSnapshot =
  | PlainTextSelectionSnapshot
  | SlotSelectionSnapshot
  | MultiSlotSelectionSnapshot;

export type DocumentEditorApplyResult =
  | { ok: true }
  | { ok: false; error: string };

export type DocumentEditorPreviewResult =
  | { ok: true; value: string; slotEdits?: EditorSlotEdit[] }
  | { ok: false; error: string };

export interface EditorSlotSelectionRange {
  slotId: string;
  startOffset: number;
  endOffset: number;
}

export interface DocumentEditorHandle {
  captureSelection: () => DocumentEditorSelectionSnapshot | null;
  previewSelectionReplacement: (
    snapshot: DocumentEditorSelectionSnapshot,
    replacementText: string
  ) => DocumentEditorPreviewResult;
  applySelectionReplacement: (
    snapshot: DocumentEditorSelectionSnapshot,
    replacementText: string
  ) => DocumentEditorApplyResult;
  /** 直接按槽位 ID 应用改写结果，跳过前端 diff 拆分。 */
  applySlotUpdates: (
    slotUpdates: Map<string, string>,
    options?: { selectionRanges?: EditorSlotSelectionRange[] }
  ) => void;
  collectSlotEdits: () => EditorSlotEdit[] | null;
}

export interface DocumentEditorProps {
  session: DocumentSession;
  value: string;
  slotOverrides: EditorSlotOverrides;
  showMarkers: boolean;
  dirty: boolean;
  busy: boolean;
  onChange: (value: string) => void;
  onChangeSlotText: (slotId: string, value: string) => void;
  onSave: () => void;
  onSelectionChange?: (hasSelection: boolean) => void;
}
