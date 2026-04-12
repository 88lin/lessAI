# Topbar Title-First Path-Secondary Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rework the workspace topbar so the document title stays the primary visual element while the full path becomes a secondary line that only uses trailing ellipsis when space runs out.

**Architecture:** Keep the existing topbar grid and button areas unchanged. Restructure the center zone in `WorkspaceBar` into a stacked container with a primary row for title plus status chips and a secondary row for the path, then move the ellipsis behavior onto dedicated text layers in CSS so the browser renders real overflow ellipses instead of content-level truncation.

**Tech Stack:** React 19, TypeScript, CSS, existing Node-based UI regression script

---

### Task 1: Lock the new topbar structure in regression checks

**Files:**
- Modify: `scripts/ui-regression.test.mjs`
- Test: `src/app/components/WorkspaceBar.tsx`
- Test: `src/styles/part-02.css`

- [ ] **Step 1: Write failing assertions for the stacked title/path layout**

```js
const workspaceBar = read("src/app/components/WorkspaceBar.tsx");

assertIncludes(workspaceBar, 'className="workspace-bar-primary-row"');
assertIncludes(workspaceBar, 'className="workspace-bar-path-line"');
assertIncludes(workspaceBar, 'className="workspace-bar-path-text"');
assertNotIncludes(workspaceBar, "workspace-bar-path-chip");

assertRule(part02, ".workspace-bar-primary-row", "display", "flex");
assertRule(part02, ".workspace-bar-path-line", "display", "flex");
assertRule(part02, ".workspace-bar-path-text", "text-overflow", "ellipsis");
```

- [ ] **Step 2: Run the regression script and verify it fails for the missing layout classes**

Run: `node scripts/ui-regression.test.mjs`

Expected: FAIL with an assertion equivalent to `期望内容包含：className="workspace-bar-primary-row"` because the center area is still a single row and the path is still rendered as a chip.

- [ ] **Step 3: Keep the old anti-regression checks that block string-level truncation**

```js
assertNotIncludes(workspaceBar, "formatTopbarTitle");
assertNotIncludes(workspaceBar, "formatTopbarPath");
```

- [ ] **Step 4: Re-run the regression script after the assertions are saved and confirm the same expected failure**

Run: `node scripts/ui-regression.test.mjs`

Expected: FAIL again for the same missing DOM/CSS structure, proving the test is checking the new requirement rather than a typo.

### Task 2: Convert the topbar center into title-first stacked content

**Files:**
- Modify: `src/app/components/WorkspaceBar.tsx`
- Modify: `src/styles/part-02.css`

- [ ] **Step 1: Change the center markup so title and chips stay in the primary row and path moves to a secondary line**

```tsx
<div className="workspace-bar-center">
  <div className="workspace-bar-primary-row">
    <strong className="workspace-bar-session" title={rawTitle}>
      <span className="workspace-bar-session-text">{rawTitle}</span>
    </strong>
    <div className="workspace-bar-chips scroll-region" data-tauri-drag-region="false">
      {/* existing StatusBadge, 模型, 应用, 进度 chip 保持原顺序 */}
    </div>
  </div>
  {currentSession ? (
    <div className="workspace-bar-path-line" title={`路径：${rawPath}`}>
      <span className="workspace-bar-path-label">路径：</span>
      <span className="workspace-bar-path-text">{rawPath}</span>
    </div>
  ) : null}
</div>
```

- [ ] **Step 2: Remove the path chip presentation from the component**

```tsx
{currentSession ? (
  <div className="workspace-bar-path-line" title={`路径：${rawPath}`}>
    <span className="workspace-bar-path-label">路径：</span>
    <span className="workspace-bar-path-text">{rawPath}</span>
  </div>
) : null}
```

- [ ] **Step 3: Update the center-zone CSS to support a stacked layout without changing the topbar grid**

```css
.workspace-bar-center {
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  gap: 4px;
}

.workspace-bar-primary-row {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 10px;
}
```

- [ ] **Step 4: Style the path as secondary copy instead of a bordered chip**

```css
.workspace-bar-path-line {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--muted);
  font-size: 0.76rem;
}

.workspace-bar-path-label {
  flex: 0 0 auto;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.workspace-bar-path-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

- [ ] **Step 5: Keep title ellipsis on the text node and leave the button area untouched**

```css
.workspace-bar-session {
  min-width: 0;
  flex: 1 1 auto;
  font-family: "Newsreader", Georgia, serif;
  font-size: 1.05rem;
}

.workspace-bar-session-text {
  display: block;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
```

- [ ] **Step 6: Preserve chip scrolling behavior but prevent it from sharing the path row**

```css
.workspace-bar-chips {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: nowrap;
  overflow-x: auto;
}
```

- [ ] **Step 7: Re-run the regression script and verify the new structure passes**

Run: `node scripts/ui-regression.test.mjs`

Expected: PASS with `[ui-regression] OK`.

### Task 3: Verify type safety and scope discipline

**Files:**
- Modify: none expected
- Test: `src/app/components/WorkspaceBar.tsx`
- Test: `src/styles/part-02.css`
- Test: `scripts/ui-regression.test.mjs`

- [ ] **Step 1: Run TypeScript validation**

Run: `pnpm run typecheck`

Expected: PASS with `tsc --noEmit` exiting 0 and no new type errors.

- [ ] **Step 2: Review the final diff to confirm only the center-zone presentation changed**

Run: `git diff -- src/app/components/WorkspaceBar.tsx src/styles/part-02.css scripts/ui-regression.test.mjs`

Expected: only the center markup, path presentation classes, and regression assertions changed; no changes to right-side button groups or the outer `.workspace-bar` grid.

- [ ] **Step 3: Run whitespace and patch hygiene verification**

Run: `git diff --check`

Expected: no diff-format errors; unrelated existing CRLF warnings may appear but there should be no new whitespace errors from these files.

- [ ] **Step 4: Commit the implementation once the checks are green**

```bash
git add src/app/components/WorkspaceBar.tsx src/styles/part-02.css scripts/ui-regression.test.mjs
git commit -m "优化顶部栏标题与路径层级"
```
