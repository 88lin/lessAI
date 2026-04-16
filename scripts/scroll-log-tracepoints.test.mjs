import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

function read(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
}

function assertIncludes(text, snippet) {
  assert.ok(text.includes(snippet), `期望内容包含：${snippet}`);
}

const appSource = read("src/App.tsx");
const paragraphFlow = read("src/stages/workbench/document/ParagraphDocumentFlow.tsx");

assertIncludes(appSource, 'logScrollRestore("refresh-session-state-start"');
assertIncludes(appSource, 'logScrollRestore("refresh-session-state-loaded"');
assertIncludes(appSource, 'logScrollRestore("tauri-chunk-completed"');
assertIncludes(appSource, 'logScrollRestore("tauri-finished"');
assertIncludes(paragraphFlow, 'logScrollRestore("paragraph-scroll-into-view"');

console.log("[scroll-log-tracepoints] OK");
