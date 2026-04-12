# Topbar Middle Ellipsis Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the workbench topbar title and path read as intentional middle ellipsis instead of hard tail truncation, without moving the existing layout.

**Architecture:** Add pure TypeScript formatting helpers in `src/lib/helpers.ts` so the truncation rule is deterministic and testable. Wire the formatted strings into `WorkspaceBar` while preserving full raw values in `title` tooltips, and keep the existing CSS overflow only as a last-resort fallback for very narrow windows.

**Tech Stack:** React 19, TypeScript, existing Node-based UI regression script

---

### Task 1: Add pure middle-ellipsis helpers with regression coverage

**Files:**
- Modify: `src/lib/helpers.ts`
- Modify: `scripts/ui-regression.test.mjs`

- [ ] **Step 1: Write the failing regression assertions in `scripts/ui-regression.test.mjs`**

```js
async function loadHelpersModule() {
  const tempRoot = join(process.cwd(), ".tmp");
  mkdirSync(tempRoot, { recursive: true });
  const dir = mkdtempSync(join(tempRoot, "lessai-helpers-"));
  const file = join(dir, "helpers.mjs");

  try {
    const source = read("src/lib/helpers.ts");
    const transpiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.ES2022,
        target: ts.ScriptTarget.ES2022
      },
      fileName: "helpers.ts"
    }).outputText;
    writeFileSync(file, transpiled, "utf8");
    return await import(pathToFileURL(file).href);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const { formatTopbarPath, formatTopbarTitle } = await loadHelpersModule();

assert.equal(
  formatTopbarTitle("04-3 作品报告（大数据应用赛，2025版）模板.docx"),
  "04-3 作品报告（大数…2025版）模板.docx"
);

assert.equal(
  formatTopbarPath("E:\\Code\\LessAI\\testdoc\\04-3 作品报告（大数据应用赛，2025版）模板.docx"),
  "E:\\Code\\LessAI\\testdoc\\…\\2025版）模板.docx"
);
```

- [ ] **Step 2: Run the regression script and verify it fails for the missing helpers**

Run: `node scripts/ui-regression.test.mjs`

Expected: FAIL with an error equivalent to `formatTopbarTitle is not a function` or an assertion mismatch.

- [ ] **Step 3: Add pure formatting helpers in `src/lib/helpers.ts`**

```ts
function middleEllipsis(value: string, headChars: number, tailChars: number) {
  if (value.length <= headChars + tailChars + 1) {
    return value;
  }
  return `${value.slice(0, headChars)}…${value.slice(-tailChars)}`;
}

export function formatTopbarTitle(title: string) {
  const value = title.trim();
  if (!value) return title;
  return middleEllipsis(value, 12, 12);
}

export function formatTopbarPath(path: string) {
  const value = formatDisplayPath(path).trim();
  if (!value) return path;

  const slashIndex = Math.max(value.lastIndexOf("/"), value.lastIndexOf("\\"));
  if (slashIndex < 0) {
    return middleEllipsis(value, 18, 18);
  }

  const separator = value[slashIndex];
  const fileName = value.slice(slashIndex + 1);
  const prefix = value.slice(0, Math.min(slashIndex, 22)).replace(/[\\/]+$/, "");
  const shortenedFileName = middleEllipsis(fileName, 6, 10);

  const shortened = `${prefix}${separator}…${separator}${shortenedFileName}`;
  return shortened.length < value.length ? shortened : middleEllipsis(value, 18, 18);
}
```

- [ ] **Step 4: Re-run the regression script and verify the new helper assertions pass**

Run: `node scripts/ui-regression.test.mjs`

Expected: PASS with no assertion failures.

### Task 2: Wire the topbar to use middle-ellipsis display text

**Files:**
- Modify: `src/app/components/WorkspaceBar.tsx`
- Modify: `src/lib/helpers.ts`

- [ ] **Step 1: Update `WorkspaceBar` imports and derive display strings once**

```tsx
import {
  formatDisplayPath,
  formatSessionStatus,
  formatTopbarPath,
  formatTopbarTitle,
  statusTone
} from "../../lib/helpers";

const rawTitle = currentSession ? currentSession.title : "未打开文档";
const displayTitle = currentSession ? formatTopbarTitle(currentSession.title) : rawTitle;
const rawPath = currentSession ? formatDisplayPath(currentSession.documentPath) : "";
const displayPath = currentSession ? formatTopbarPath(currentSession.documentPath) : "";
```

- [ ] **Step 2: Render formatted text but keep the full raw values in `title`**

```tsx
<strong className="workspace-bar-session" title={rawTitle}>
  {displayTitle}
</strong>

{currentSession ? (
  <span className="context-chip" title={`路径：${rawPath}`}>
    路径：{displayPath}
  </span>
) : null}
```

- [ ] **Step 3: Keep layout stable and do not widen or shrink any topbar controls**

```tsx
<div className="workspace-bar-center">
  <strong className="workspace-bar-session" title={rawTitle}>
    {displayTitle}
  </strong>
  <div className="workspace-bar-chips scroll-region" data-tauri-drag-region="false">
    {/* existing chips stay in the same order */}
  </div>
</div>
```

- [ ] **Step 4: Run TypeScript validation**

Run: `pnpm run typecheck`

Expected: PASS with no new type errors from `WorkspaceBar` or `helpers.ts`.

### Task 3: Re-verify topbar behavior and guard against visual regressions

**Files:**
- Modify: `scripts/ui-regression.test.mjs`
- Test: `src/app/components/WorkspaceBar.tsx`
- Test: `src/styles/part-01.css`
- Test: `src/styles/part-02.css`

- [ ] **Step 1: Add a static markup assertion that the topbar still renders the same chip structure**

```js
const workspaceBar = read("src/app/components/WorkspaceBar.tsx");
assertIncludes(workspaceBar, 'className="workspace-bar-session"');
assertIncludes(workspaceBar, 'title={rawTitle}');
assertIncludes(workspaceBar, '路径：{displayPath}');
```

- [ ] **Step 2: Re-run the UI regression script**

Run: `node scripts/ui-regression.test.mjs`

Expected: PASS with no CSS or markup regression failures.

- [ ] **Step 3: Review the final diff to ensure only title/path display behavior changed**

Run: `git diff -- src/lib/helpers.ts src/app/components/WorkspaceBar.tsx scripts/ui-regression.test.mjs`

Expected: only helper logic, topbar text wiring, and regression assertions changed; no button sizing or topbar layout structure changes.
