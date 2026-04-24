import assert from "node:assert/strict";

import { read } from "./test-helpers.mjs";

const rustSource = read("src-tauri/src/adapters/docx/placeholders.rs");
const frontendSource = read("src/lib/protectedText.tsx");

const backendLabels = [...rustSource.matchAll(/DOCX_[A-Z_]+_PLACEHOLDER:\s*&str\s*=\s*"\[([^\]"]+)\]";/g)].map(
  (match) => match[1]
);
assert.ok(backendLabels.length > 0, "后端 placeholders.rs 未解析到占位符常量");

const frontendListMatch = frontendSource.match(
  /const DOCX_PLACEHOLDER_LABELS = \[([\s\S]*?)\] as const;/
);
assert.ok(frontendListMatch, "前端 protectedText.tsx 缺少 DOCX_PLACEHOLDER_LABELS 列表");
const frontendLabels = [...frontendListMatch[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);

function normalized(values) {
  return [...new Set(values)].sort();
}

function onlyIn(left, right) {
  const rightSet = new Set(right);
  return left.filter((value) => !rightSet.has(value));
}

const backendNormalized = normalized(backendLabels);
const frontendNormalized = normalized(frontendLabels);
const onlyBackend = onlyIn(backendNormalized, frontendNormalized);
const onlyFrontend = onlyIn(frontendNormalized, backendNormalized);

assert.deepEqual(
  frontendNormalized,
  backendNormalized,
  `DOCX 占位符标签不一致。仅后端: ${onlyBackend.join(", ") || "(无)"}；仅前端: ${
    onlyFrontend.join(", ") || "(无)"
  }`
);

console.log("[docx-placeholder-sync] OK");
