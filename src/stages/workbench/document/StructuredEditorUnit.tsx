import { Fragment, memo } from "react";

import type { EditorSlotOverrides } from "../../../lib/editorSlots";
import { resolveEditorSlotText } from "../../../lib/editorSlots";
import { resolveRewriteUnitSlots } from "../../../lib/helpers";
import type { DocumentSession, RewriteUnit } from "../../../lib/types";
import { EditableSlotSpan, slotPresentationClass } from "./structuredEditorShared";

interface StructuredEditorUnitProps {
  session: DocumentSession;
  rewriteUnit: RewriteUnit;
  slotOverrides: EditorSlotOverrides;
  registerNode: (slotId: string, node: HTMLSpanElement | null) => void;
}

function buildUnitClassName(hasEditableSlot: boolean) {
  return [
    "doc-unit",
    "doc-paragraph-unit",
    "structured-editor-unit",
    hasEditableSlot ? "" : "is-protected"
  ]
    .filter(Boolean)
    .join(" ");
}

const slotClassOptions = {
  baseClassName: "doc-paragraph-fragment structured-editor-slot",
  protectedClassName: "is-fragment-protected is-locked"
} as const;

export const StructuredEditorUnit = memo(function StructuredEditorUnit({
  session,
  rewriteUnit,
  slotOverrides,
  registerNode
}: StructuredEditorUnitProps) {
  const slots = resolveRewriteUnitSlots(session, rewriteUnit);
  if (slots.length === 0) {
    return null;
  }

  const hasEditableSlot = slots.some((slot) => slot.editable);
  const trailingSeparator = slots[slots.length - 1]?.separatorAfter ?? "";

  return (
    <span className="doc-unit-wrap">
      <span className={buildUnitClassName(hasEditableSlot)} data-rewrite-unit-id={rewriteUnit.id}>
        {slots.map((slot, index) => {
          const text = resolveEditorSlotText(slot, slotOverrides);
          const intraUnitSeparator = index < slots.length - 1 ? slot.separatorAfter : "";
          return (
            <Fragment key={slot.id}>
              {slot.editable ? (
                <EditableSlotSpan
                  slot={slot}
                  text={text}
                  registerNode={registerNode}
                  classNameOptions={slotClassOptions}
                />
              ) : (
                <span
                  className={slotPresentationClass(slot, slotClassOptions)}
                  contentEditable={false}
                >
                  {text}
                </span>
              )}
              {intraUnitSeparator ? (
                <span className="structured-editor-separator" contentEditable={false}>
                  {intraUnitSeparator}
                </span>
              ) : null}
            </Fragment>
          );
        })}
      </span>
      {trailingSeparator ? (
        <span
          className="doc-unit-separator structured-editor-separator"
          contentEditable={false}
        >
          {trailingSeparator}
        </span>
      ) : null}
    </span>
  );
});
