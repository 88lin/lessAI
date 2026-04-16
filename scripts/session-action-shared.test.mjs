import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import assert from "node:assert/strict";
import ts from "typescript";

function read(path) {
  return readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
}

async function loadSessionActionSharedModule() {
  const tempRoot = join(process.cwd(), ".tmp");
  mkdirSync(tempRoot, { recursive: true });
  const dir = mkdtempSync(join(tempRoot, "lessai-session-action-shared-"));
  const hooksDir = join(dir, "src", "app", "hooks");
  const libDir = join(dir, "src", "lib");
  const file = join(hooksDir, "sessionActionShared.mjs");
  const helpersFile = join(libDir, "helpers.mjs");

  try {
    mkdirSync(hooksDir, { recursive: true });
    mkdirSync(libDir, { recursive: true });

    const helpersSource = read("src/lib/helpers.ts");
    const transpiledHelpers = ts.transpileModule(helpersSource, {
      compilerOptions: {
        module: ts.ModuleKind.ES2022,
        target: ts.ScriptTarget.ES2022
      },
      fileName: "helpers.ts"
    }).outputText;
    writeFileSync(helpersFile, transpiledHelpers, "utf8");

    const source = read("src/app/hooks/sessionActionShared.ts");
    const transpiled = ts
      .transpileModule(source, {
        compilerOptions: {
          module: ts.ModuleKind.ES2022,
          target: ts.ScriptTarget.ES2022
        },
        fileName: "sessionActionShared.ts"
      })
      .outputText.replaceAll('"../../lib/helpers"', '"../../lib/helpers.mjs"');
    writeFileSync(file, transpiled, "utf8");
    return await import(pathToFileURL(file).href);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

function sampleSession(id = "session-1") {
  return {
    id,
    title: "示例",
    documentPath: "/tmp/example.txt",
    sourceText: "正文",
    sourceSnapshot: null,
    normalizedText: "正文",
    writeBackSupported: true,
    writeBackBlockReason: null,
    plainTextEditorSafe: true,
    plainTextEditorBlockReason: null,
    chunkPreset: "paragraph",
    rewriteHeadings: false,
    chunks: [],
    suggestions: [],
    nextSuggestionSequence: 1,
    status: "idle",
    createdAt: "2026-04-15T00:00:00.000Z",
    updatedAt: "2026-04-15T00:00:00.000Z"
  };
}

const {
  ensureAllowedOrNotify,
  refreshAllowedSessionOrNotify,
  refreshRewriteableSessionOrNotify,
  refreshSessionOrNotify
} =
  await loadSessionActionSharedModule();

{
  const notices = [];
  const refreshed = await refreshSessionOrNotify({
    session: sampleSession("session-success"),
    refreshSessionState: async () => sampleSession("session-refreshed"),
    options: { preserveChunk: true, preserveSuggestion: true },
    showNotice: (tone, message) => notices.push({ tone, message }),
    errorPrefix: "执行失败",
    formatError: (error) => String(error)
  });

  assert.equal(refreshed?.id, "session-refreshed");
  assert.deepEqual(notices, []);
}

{
  const notices = [];
  const refreshed = await refreshAllowedSessionOrNotify({
    session: sampleSession("session-allowed"),
    refreshSessionState: async () => sampleSession("session-latest"),
    showNotice: (tone, message) => notices.push({ tone, message }),
    errorPrefix: "执行失败",
    formatError: (error) => String(error),
    allowed: (session) => session.writeBackSupported,
    blockedMessage: (session) => session.writeBackBlockReason,
    fallbackMessage: "当前文档暂不支持安全写回覆盖。"
  });

  assert.equal(refreshed?.id, "session-latest");
  assert.deepEqual(notices, []);
}

{
  const notices = [];
  const refreshed = await refreshRewriteableSessionOrNotify({
    session: sampleSession("session-rewriteable"),
    refreshSessionState: async () => sampleSession("session-rewriteable-latest"),
    showNotice: (tone, message) => notices.push({ tone, message }),
    errorPrefix: "执行失败",
    formatError: (error) => String(error)
  });

  assert.equal(refreshed?.id, "session-rewriteable-latest");
  assert.deepEqual(notices, []);
}

{
  const notices = [];
  const refreshed = await refreshAllowedSessionOrNotify({
    session: sampleSession("session-blocked"),
    refreshSessionState: async () => ({
      ...sampleSession("session-blocked-latest"),
      writeBackSupported: false,
      writeBackBlockReason: "已阻止"
    }),
    showNotice: (tone, message) => notices.push({ tone, message }),
    errorPrefix: "执行失败",
    formatError: (error) => String(error),
    allowed: (session) => session.writeBackSupported,
    blockedMessage: (session) => session.writeBackBlockReason,
    fallbackMessage: "当前文档暂不支持安全写回覆盖。"
  });

  assert.equal(refreshed, null);
  assert.deepEqual(notices, [{ tone: "warning", message: "已阻止" }]);
}

{
  const notices = [];
  const refreshed = await refreshRewriteableSessionOrNotify({
    session: sampleSession("session-rewriteable-blocked"),
    refreshSessionState: async () => ({
      ...sampleSession("session-rewriteable-blocked-latest"),
      writeBackSupported: false,
      writeBackBlockReason: "不可改写"
    }),
    showNotice: (tone, message) => notices.push({ tone, message }),
    errorPrefix: "执行失败",
    formatError: (error) => String(error)
  });

  assert.equal(refreshed, null);
  assert.deepEqual(notices, [{ tone: "warning", message: "不可改写" }]);
}

{
  const notices = [];
  const refreshed = await refreshAllowedSessionOrNotify({
    session: sampleSession("session-refresh-fail"),
    refreshSessionState: async () => {
      throw new Error("network down");
    },
    showNotice: (tone, message) => notices.push({ tone, message }),
    errorPrefix: "写回失败",
    formatError: (error) => String(error instanceof Error ? error.message : error),
    allowed: () => true,
    blockedMessage: () => null,
    fallbackMessage: "不应出现"
  });

  assert.equal(refreshed, null);
  assert.deepEqual(notices, [{ tone: "error", message: "写回失败：network down" }]);
}

{
  const notices = [];
  const refreshed = await refreshSessionOrNotify({
    session: sampleSession("session-fail"),
    refreshSessionState: async () => {
      throw new Error("network down");
    },
    showNotice: (tone, message) => notices.push({ tone, message }),
    errorPrefix: "写回失败",
    formatError: (error) => String(error instanceof Error ? error.message : error)
  });

  assert.equal(refreshed, null);
  assert.deepEqual(notices, [{ tone: "error", message: "写回失败：network down" }]);
}

{
  const notices = [];
  const allowed = ensureAllowedOrNotify({
    allowed: true,
    blockedMessage: "不应出现",
    fallbackMessage: "不应出现",
    showNotice: (tone, message) => notices.push({ tone, message })
  });

  assert.equal(allowed, true);
  assert.deepEqual(notices, []);
}

{
  const notices = [];
  const allowed = ensureAllowedOrNotify({
    allowed: false,
    blockedMessage: null,
    fallbackMessage: "当前文档暂不支持安全写回覆盖。",
    showNotice: (tone, message) => notices.push({ tone, message })
  });

  assert.equal(allowed, false);
  assert.deepEqual(notices, [
    { tone: "warning", message: "当前文档暂不支持安全写回覆盖。" }
  ]);
}
