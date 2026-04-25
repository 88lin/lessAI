import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import assert from "node:assert/strict";
import ts from "typescript";
import { read } from "./test-helpers.mjs";

async function loadDocumentScrollRestoreSharedModule() {
  const tempRoot = join(process.cwd(), ".tmp");
  mkdirSync(tempRoot, { recursive: true });
  const dir = mkdtempSync(join(tempRoot, "lessai-document-scroll-restore-"));
  const file = join(dir, "documentScrollRestoreShared.mjs");

  try {
    const source = read("src/app/hooks/documentScrollRestoreShared.ts");
    const transpiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.ES2022,
        target: ts.ScriptTarget.ES2022
      },
      fileName: "documentScrollRestoreShared.ts"
    }).outputText;
    writeFileSync(file, transpiled, "utf8");
    return await import(pathToFileURL(file).href);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const {
  SCROLL_RESTORE_TOLERANCE_PX,
  REQUIRED_SCROLL_STABLE_FRAMES,
  MAX_SCROLL_RESTORE_ATTEMPTS,
  beginScrollRestore,
  advanceScrollRestore
} = await loadDocumentScrollRestoreSharedModule();

{
  const started = beginScrollRestore(240);
  assert.deepEqual(started, {
    targetScrollTop: 240,
    attempts: 0,
    stableFrames: 0
  });
}

{
  const started = beginScrollRestore(240);
  const progressed = advanceScrollRestore(started, 120);
  assert.equal(progressed.done, false);
  assert.deepEqual(progressed.next, {
    targetScrollTop: 240,
    attempts: 1,
    stableFrames: 0
  });
}

{
  let progress = beginScrollRestore(240);
  for (let frame = 0; frame < REQUIRED_SCROLL_STABLE_FRAMES - 1; frame += 1) {
    const next = advanceScrollRestore(progress, 240 + SCROLL_RESTORE_TOLERANCE_PX / 2);
    assert.equal(next.done, false);
    progress = next.next;
  }

  const completed = advanceScrollRestore(progress, 240);
  assert.equal(completed.done, true);
  assert.equal(completed.next.stableFrames, REQUIRED_SCROLL_STABLE_FRAMES);
}

{
  const started = {
    targetScrollTop: 240,
    attempts: 2,
    stableFrames: REQUIRED_SCROLL_STABLE_FRAMES - 1
  };
  const reset = advanceScrollRestore(
    started,
    240 + SCROLL_RESTORE_TOLERANCE_PX + 5
  );
  assert.equal(reset.done, false);
  assert.equal(reset.next.stableFrames, 0);
}

{
  const nearLimit = {
    targetScrollTop: 240,
    attempts: MAX_SCROLL_RESTORE_ATTEMPTS - 1,
    stableFrames: 0
  };
  const completed = advanceScrollRestore(nearLimit, 0);
  assert.equal(completed.done, true);
  assert.equal(completed.next.attempts, MAX_SCROLL_RESTORE_ATTEMPTS);
}

console.log("[document-scroll-restore] OK");
