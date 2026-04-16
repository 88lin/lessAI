import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

function read(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
}

function assertIncludes(text, snippet) {
  assert.ok(text.includes(snippet), `期望内容包含：${snippet}`);
}

function assertNotIncludes(text, snippet) {
  assert.ok(!text.includes(snippet), `期望内容不包含：${snippet}`);
}

const source = read("src/app/hooks/useDocumentFinalizeActions.ts");

assertIncludes(source, "activeChunkIndexRef: React.MutableRefObject<number>;");
assertIncludes(source, "Math.min(activeChunkIndexRef.current, Math.max(0, reopened.chunks.length - 1))");
assertIncludes(source, "Math.min(activeChunkIndexRef.current, Math.max(0, refreshed.chunks.length - 1))");
assertIncludes(source, "Math.min(activeChunkIndexRef.current, Math.max(0, rebuilt.chunks.length - 1))");
assertNotIncludes(source, "applySessionState(reopened, selectDefaultChunkIndex(reopened))");
assertNotIncludes(source, "applySessionState(refreshed, selectDefaultChunkIndex(refreshed))");
assertNotIncludes(source, "applySessionState(rebuilt, selectDefaultChunkIndex(rebuilt))");

console.log("[finalize-preserve-active-chunk] OK");
