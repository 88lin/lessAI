import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

function read(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
}

function assertIncludes(text, snippet) {
  assert.ok(text.includes(snippet), `期望内容包含：${snippet}`);
}

const appSource = read("src/App.tsx");
const sessionActionShared = read("src/app/hooks/sessionActionShared.ts");
const rewriteActions = read("src/app/hooks/useRewriteActions.ts");
const suggestionActions = read("src/app/hooks/useSuggestionActions.ts");

assertIncludes(sessionActionShared, "preserveScroll?: boolean;");
assertIncludes(sessionActionShared, "preservedScrollTop?: number | null");

assertIncludes(appSource, "options?.preserveScroll === false ? undefined : captureDocumentScrollPosition()");
assertIncludes(appSource, "options.preservedScrollTop ?? null");

assertIncludes(rewriteActions, "captureDocumentScrollPosition: () => number | null;");
assertIncludes(rewriteActions, "const preservedScrollTop = captureDocumentScrollPosition();");
assertIncludes(rewriteActions, "preservedScrollTop");

assertIncludes(suggestionActions, "captureDocumentScrollPosition: () => number | null;");
assertIncludes(suggestionActions, "const preservedScrollTop = captureDocumentScrollPosition();");
assertIncludes(suggestionActions, "preservedScrollTop");

console.log("[workbench-scroll-regression] OK");
