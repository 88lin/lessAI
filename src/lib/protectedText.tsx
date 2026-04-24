import type { ReactNode } from "react";

import { splitMarkdownInlineProtected } from "./markdownProtectedSegments";
import { fileExtensionLower } from "./path";
import { type ProtectedSegment } from "./protectedTextShared";
import { splitTexInlineProtected } from "./texProtectedSegments";
import type { WritebackSlot } from "./types";

export type ClientDocumentFormat = "plain" | "markdown" | "tex" | "docx";

export function guessClientDocumentFormat(documentPath: string): ClientDocumentFormat {
  const ext = fileExtensionLower(documentPath ?? "");

  if (ext === "md" || ext === "markdown") return "markdown";
  if (ext === "tex" || ext === "latex") return "tex";
  if (ext === "docx") return "docx";
  return "plain";
}

export function renderInlineProtectedText(
  text: string,
  format: ClientDocumentFormat,
  keyPrefix = "protected",
  options?: { slot?: WritebackSlot | null }
): ReactNode {
  const segments = resolveProtectedSegments(text, format, options);
  if (!segments || (segments.length === 1 && segments[0].kind === "text")) return text;

  return segments.map((segment, index) => {
    if (segment.kind === "text") return segment.text;
    return (
      <span
        key={`${keyPrefix}-${index}-${segment.text.length}`}
        className="inline-protected"
        data-protect-kind={segment.protectKind}
        title={`保护区：${segment.label}，AI 不会修改`}
      >
        {segment.text}
      </span>
    );
  });
}

function resolveProtectedSegments(
  text: string,
  format: ClientDocumentFormat,
  options?: { slot?: WritebackSlot | null }
): ProtectedSegment[] | null {
  if (format === "markdown") {
    const likelyHasProtected =
      text.includes("`") ||
      text.includes("$") ||
      text.includes("[") ||
      text.includes("!") ||
      text.includes("<") ||
      text.includes("http://") ||
      text.includes("https://") ||
      text.includes("www.");
    return likelyHasProtected ? splitMarkdownInlineProtected(text) : null;
  }

  if (format === "tex") {
    const likelyHasProtected = text.includes("\\") || text.includes("$") || text.includes("%");
    return likelyHasProtected ? splitTexInlineProtected(text) : null;
  }

  if (format === "docx") {
    const slotProtectKind = options?.slot?.presentation?.protectKind;
    if (slotProtectKind) {
      return [
        {
          kind: "protected",
          text,
          label: "DOCX 保护片段",
          protectKind: slotProtectKind
        }
      ];
    }
    return splitDocxPlaceholders(text);
  }

  return null;
}

const DOCX_PLACEHOLDER_LABELS = [
  "图片",
  "文本框",
  "图表",
  "图形",
  "组合图形",
  "内容控件",
  "表格",
  "分节符",
  "字段",
  "分页符"
] as const;

const DOCX_PLACEHOLDER_PATTERN = new RegExp(
  `\\[(?:${DOCX_PLACEHOLDER_LABELS.map(escapeRegExpLiteral).join("|")})\\]`,
  "g"
);

function escapeRegExpLiteral(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function splitDocxPlaceholders(text: string): ProtectedSegment[] | null {
  if (!text.includes("[")) return null;

  const segments: ProtectedSegment[] = [];
  let cursor = 0;

  for (const match of text.matchAll(DOCX_PLACEHOLDER_PATTERN)) {
    const full = match[0];
    const start = match.index;
    if (start == null) continue;
    if (start > cursor) {
      segments.push({ kind: "text", text: text.slice(cursor, start) });
    }
    segments.push({
      kind: "protected",
      text: full,
      label: "DOCX 占位符",
      protectKind: "docx-placeholder"
    });
    cursor = start + full.length;
  }

  if (segments.length === 0) return null;
  if (cursor < text.length) {
    segments.push({ kind: "text", text: text.slice(cursor) });
  }
  return segments;
}
