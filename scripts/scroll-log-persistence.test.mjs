import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

function read(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
}

function assertIncludes(text, snippet) {
  assert.ok(text.includes(snippet), `期望内容包含：${snippet}`);
}

const mainRs = read("src-tauri/src/main.rs");
const debugLogger = read("src/app/hooks/documentScrollRestoreDebug.ts");

assertIncludes(mainRs, "TargetKind::LogDir");
assertIncludes(mainRs, "TargetKind::Stdout");
assertIncludes(mainRs, "TargetKind::Webview");
assertIncludes(debugLogger, 'import { info } from "@tauri-apps/plugin-log";');
assertIncludes(debugLogger, "void info(");
assertIncludes(debugLogger, "JSON.stringify(detail)");

console.log("[scroll-log-persistence] OK");
