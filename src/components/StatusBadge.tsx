import { memo, type ReactNode } from "react";
import type { NoticeTone } from "../lib/constants";

export const StatusBadge = memo(function StatusBadge({
  tone,
  children
}: {
  tone: NoticeTone;
  children: ReactNode;
}) {
  return <span className={`status-badge is-${tone}`}>{children}</span>;
});
