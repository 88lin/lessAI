import type { DocumentSession, RewriteUnit, WritebackSlot } from "./types";

export function buildWritebackSlotMap(slots: ReadonlyArray<WritebackSlot>) {
  return new Map(slots.map((slot) => [slot.id, slot] as const));
}

export function resolveSlotsByIds(
  slots: ReadonlyArray<WritebackSlot>,
  slotIds: ReadonlyArray<string>
) {
  const slotMap = buildWritebackSlotMap(slots);
  return slotIds
    .map((slotId) => slotMap.get(slotId))
    .filter((slot): slot is WritebackSlot => slot != null);
}

export function mergedTextFromSlots(slots: ReadonlyArray<WritebackSlot>) {
  return slots.map((slot) => `${slot.text}${slot.separatorAfter}`).join("");
}

export function mergedTextFromSlotIds(
  slots: ReadonlyArray<WritebackSlot>,
  slotIds: ReadonlyArray<string>
) {
  return mergedTextFromSlots(resolveSlotsByIds(slots, slotIds));
}

export function rewriteUnitSourceText(session: DocumentSession, rewriteUnit: RewriteUnit) {
  return mergedTextFromSlotIds(session.writebackSlots, rewriteUnit.slotIds);
}
