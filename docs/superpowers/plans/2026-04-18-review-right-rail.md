# Review Right Rail Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the workbench review right rail into a compact suggestion list that uses hover actions instead of an always-open diff/detail panel, while keeping the left document pane as the single source of truth for actual rewrite content.

**Architecture:** Add one pure row-model helper module and one focused row component so the new right-rail behavior stays testable and the existing review pane stays small. Remove the obsolete `reviewView` state chain entirely, move per-suggestion actions into list rows, keep the review header status-only, and replace the old diff/switch layout CSS with compact hover-action list styles.

**Tech Stack:** React 19, TypeScript, existing Node-based `ui-regression` script, existing Tauri/workbench hooks

---

## File Map

- `src/stages/workbench/review/reviewSuggestionRowModel.ts`
  Pure helpers for row title text, secondary action label, and disabled/busy action-state calculation.
- `src/stages/workbench/review/ReviewSuggestionRow.tsx`
  Presentational row component for one suggestion line, including hover actions and the `···` menu.
- `src/stages/workbench/review/SuggestionReviewPane.tsx`
  Rebuilt as summary strip + compact list; no more diff/source/candidate viewer.
- `src/stages/workbench/review/ReviewActionBar.tsx`
  Reduced to a status-only header chip row; no per-suggestion action buttons.
- `src/stages/workbench/ReviewPanel.tsx`
  Passes busy/action props into the list pane instead of the old action bar.
- `src/stages/WorkbenchStage.tsx`
  Stops threading the obsolete `reviewView` prop through the stage tree.
- `src/App.tsx`
  Deletes `reviewView` state and removes all reset calls that only existed for the old detail viewer.
- `src/app/hooks/useSuggestionActions.ts`
  Stops forcing `reviewView = "diff"` on suggestion selection.
- `src/app/hooks/useRewriteActions.ts`
  Removes `setReviewView` from hook options and cleanup paths.
- `src/app/hooks/useDocumentActions.ts`
  Removes `setReviewView` from document/session refresh flows.
- `src/app/hooks/useDocumentFinalizeActions.ts`
  Removes `setReviewView` from finalize/writeback flows.
- `src/lib/constants.ts`
  Deletes `ReviewView` and `REVIEW_VIEW_OPTIONS`.
- `src/styles/part-02.css`
  Removes old right-rail max-height/detail assumptions and keeps the review body as a vertical list host.
- `src/styles/part-04.css`
  Adds compact row, hover-action, and secondary-menu styles; removes unused detail/switch styles.
- `scripts/ui-regression.test.mjs`
  Adds regression coverage for the new row-model helpers, new right-rail markup, and removal of the old `reviewView` chain.

### Task 1: Add a pure row-model helper with regression coverage

**Files:**
- Create: `src/stages/workbench/review/reviewSuggestionRowModel.ts`
- Modify: `scripts/ui-regression.test.mjs`
- Test: `src/lib/types.ts`

- [ ] **Step 1: Add failing regression coverage for the new row-model module**

```js
async function loadReviewSuggestionRowModel() {
  const tempRoot = join(process.cwd(), ".tmp");
  mkdirSync(tempRoot, { recursive: true });
  const dir = mkdtempSync(join(tempRoot, "lessai-review-row-model-"));

  try {
    const source = read("src/stages/workbench/review/reviewSuggestionRowModel.ts");
    const transpiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.ES2022,
        target: ts.ScriptTarget.ES2022
      },
      fileName: "reviewSuggestionRowModel.ts"
    }).outputText;
    const rewritten = rewriteRelativeImports(transpiled);
    writeFileSync(join(dir, "reviewSuggestionRowModel.mjs"), rewritten, "utf8");

    return await import(pathToFileURL(join(dir, "reviewSuggestionRowModel.mjs")).href);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const {
  buildSuggestionRowActionState,
  buildSuggestionRowSecondaryActionLabel,
  buildSuggestionRowTitle
} = await loadReviewSuggestionRowModel();

const sampleSuggestion = {
  id: "sg-1",
  sequence: 12,
  rewriteUnitId: "unit-1",
  beforeText: "手工统计问卷结果",
  afterText: "自动汇总问卷结果，压缩后半句长度",
  diffSpans: [],
  decision: "applied",
  slotUpdates: [],
  createdAt: "2026-04-18T10:42:00.000Z",
  updatedAt: "2026-04-18T10:42:00.000Z"
};

assert.equal(
  buildSuggestionRowTitle(sampleSuggestion, 40),
  "#12 自动汇总问卷结果，压缩后半句长度"
);

assert.equal(buildSuggestionRowSecondaryActionLabel("applied"), "取消应用");
assert.equal(buildSuggestionRowSecondaryActionLabel("proposed"), "忽略");

assert.deepEqual(
  buildSuggestionRowActionState({
    suggestionId: "sg-1",
    decision: "applied",
    busyAction: null,
    anyBusy: false,
    editorMode: false,
    rewriteRunning: false,
    rewritePaused: false,
    settingsReady: true,
    rewriteUnitFailed: true
  }),
  {
    applyBusy: false,
    applyDisabled: true,
    deleteBusy: false,
    deleteDisabled: false,
    dismissBusy: false,
    dismissDisabled: false,
    retryBusy: false,
    retryDisabled: false,
    retryVisible: true
  }
);
```

- [ ] **Step 2: Run the regression script and verify it fails because the helper file does not exist yet**

Run: `node scripts/ui-regression.test.mjs`

Expected: FAIL with an error equivalent to `ENOENT: no such file or directory, open 'src/stages/workbench/review/reviewSuggestionRowModel.ts'`.

- [ ] **Step 3: Create `reviewSuggestionRowModel.ts` with the row text and action-state helpers**

```ts
import type { RewriteSuggestion, SuggestionDecision } from "../../../lib/types";

interface ReviewSuggestionRowActionStateInput {
  suggestionId: string;
  decision: SuggestionDecision;
  busyAction: string | null;
  anyBusy: boolean;
  editorMode: boolean;
  rewriteRunning: boolean;
  rewritePaused: boolean;
  settingsReady: boolean;
  rewriteUnitFailed: boolean;
}

export interface ReviewSuggestionRowActionState {
  applyBusy: boolean;
  applyDisabled: boolean;
  deleteBusy: boolean;
  deleteDisabled: boolean;
  dismissBusy: boolean;
  dismissDisabled: boolean;
  retryBusy: boolean;
  retryDisabled: boolean;
  retryVisible: boolean;
}

function compactWhitespace(value: string) {
  return value.replace(/\s+/g, " ").trim();
}

function ellipsis(value: string, maxChars: number) {
  return value.length > maxChars ? `${value.slice(0, maxChars)}…` : value;
}

export function buildSuggestionRowTitle(
  suggestion: RewriteSuggestion,
  maxChars = 32
) {
  const preferred =
    compactWhitespace(suggestion.afterText) ||
    compactWhitespace(suggestion.beforeText) ||
    "（空片段）";
  return `#${suggestion.sequence} ${ellipsis(preferred, maxChars)}`;
}

export function buildSuggestionRowSecondaryActionLabel(decision: SuggestionDecision) {
  return decision === "applied" ? "取消应用" : "忽略";
}

export function buildSuggestionRowActionState(
  input: ReviewSuggestionRowActionStateInput
): ReviewSuggestionRowActionState {
  const sharedBlocked =
    input.editorMode || input.rewriteRunning || input.rewritePaused;
  const applyBusy = input.busyAction === `apply-suggestion:${input.suggestionId}`;
  const deleteBusy = input.busyAction === `delete-suggestion:${input.suggestionId}`;
  const dismissBusy = input.busyAction === `dismiss-suggestion:${input.suggestionId}`;
  const retryBusy = input.busyAction === "retry-rewrite-unit";

  return {
    applyBusy,
    applyDisabled:
      sharedBlocked ||
      input.decision === "applied" ||
      applyBusy ||
      (input.anyBusy && !applyBusy),
    deleteBusy,
    deleteDisabled:
      sharedBlocked || deleteBusy || (input.anyBusy && !deleteBusy),
    dismissBusy,
    dismissDisabled:
      sharedBlocked ||
      input.decision === "dismissed" ||
      dismissBusy ||
      (input.anyBusy && !dismissBusy),
    retryBusy,
    retryDisabled:
      !input.rewriteUnitFailed ||
      !input.settingsReady ||
      sharedBlocked ||
      retryBusy ||
      (input.anyBusy && !retryBusy),
    retryVisible: input.rewriteUnitFailed
  };
}
```

- [ ] **Step 4: Re-run the regression script and verify the helper assertions pass**

Run: `node scripts/ui-regression.test.mjs`

Expected: PASS for the new helper assertions, while later tasks still fail for the old right-rail markup.

- [ ] **Step 5: Commit the helper baseline**

```bash
git add scripts/ui-regression.test.mjs src/stages/workbench/review/reviewSuggestionRowModel.ts
git commit -m "提取审阅列表行模型"
```

### Task 2: Rebuild the review pane around compact rows and row-level actions

**Files:**
- Create: `src/stages/workbench/review/ReviewSuggestionRow.tsx`
- Modify: `src/stages/workbench/review/SuggestionReviewPane.tsx`
- Modify: `src/stages/workbench/review/ReviewActionBar.tsx`
- Modify: `src/stages/workbench/ReviewPanel.tsx`
- Modify: `scripts/ui-regression.test.mjs`

- [ ] **Step 1: Add failing static assertions for the new row-driven markup and removal of the old detail viewer**

```js
const reviewPanel = read("src/stages/workbench/ReviewPanel.tsx");
const suggestionReviewPane = read("src/stages/workbench/review/SuggestionReviewPane.tsx");
const reviewActionBar = read("src/stages/workbench/review/ReviewActionBar.tsx");

assertIncludes(suggestionReviewPane, 'className="review-summary-strip"');
assertIncludes(suggestionReviewPane, "<ReviewSuggestionRow");
assertNotIncludes(suggestionReviewPane, 'className="diff-view"');
assertNotIncludes(suggestionReviewPane, "REVIEW_VIEW_OPTIONS");
assertIncludes(reviewActionBar, 'className="workbench-review-actionbar-status"');
assertNotIncludes(reviewActionBar, "onApplySuggestion");
assertNotIncludes(reviewActionBar, "onDismissSuggestion");
assertNotIncludes(reviewActionBar, "onDeleteSuggestion");
assertIncludes(reviewPanel, "onApplySuggestion={onApplySuggestion}");
assertIncludes(reviewPanel, "onDeleteSuggestion={onDeleteSuggestion}");
```

- [ ] **Step 2: Run the regression script and verify it fails against the old review pane**

Run: `node scripts/ui-regression.test.mjs`

Expected: FAIL because `SuggestionReviewPane.tsx` still contains `diff-view` / `REVIEW_VIEW_OPTIONS` and does not render `ReviewSuggestionRow`.

- [ ] **Step 3: Create `ReviewSuggestionRow.tsx` to own one compact list row**

```tsx
import { memo } from "react";
import { Check, LoaderCircle, MoreHorizontal, RotateCcw, Trash2, X } from "lucide-react";
import { StatusBadge } from "../../../components/StatusBadge";
import { formatDate, formatSuggestionDecision, suggestionTone } from "../../../lib/helpers";
import type { RewriteSuggestion } from "../../../lib/types";
import type { ReviewSuggestionRowActionState } from "./reviewSuggestionRowModel";
import {
  buildSuggestionRowSecondaryActionLabel,
  buildSuggestionRowTitle
} from "./reviewSuggestionRowModel";

interface ReviewSuggestionRowProps {
  suggestion: RewriteSuggestion;
  active: boolean;
  menuOpen: boolean;
  actionState: ReviewSuggestionRowActionState;
  onSelect: () => void;
  onApply: () => void;
  onDelete: () => void;
  onDismiss: () => void;
  onRetry: () => void;
  onToggleMenu: () => void;
}

export const ReviewSuggestionRow = memo(function ReviewSuggestionRow({
  suggestion,
  active,
  menuOpen,
  actionState,
  onSelect,
  onApply,
  onDelete,
  onDismiss,
  onRetry,
  onToggleMenu
}: ReviewSuggestionRowProps) {
  const secondaryLabel = buildSuggestionRowSecondaryActionLabel(suggestion.decision);

  return (
    <div className={`review-suggestion-row ${active ? "is-active" : ""}`}>
      <button
        type="button"
        className="review-suggestion-row-main"
        onClick={onSelect}
        title={buildSuggestionRowTitle(suggestion, 80)}
      >
        <div className="review-suggestion-row-mainline">
          <span className="review-suggestion-row-title">
            {buildSuggestionRowTitle(suggestion)}
          </span>
          <StatusBadge tone={suggestionTone(suggestion.decision)}>
            {formatSuggestionDecision(suggestion.decision)}
          </StatusBadge>
        </div>
        <span className="review-suggestion-row-meta">
          {formatDate(suggestion.createdAt)} · {formatSuggestionDecision(suggestion.decision)}
        </span>
      </button>

      <div className="review-suggestion-row-actions">
        <button
          type="button"
          className="icon-button icon-button-sm review-suggestion-row-action is-apply"
          onClick={onApply}
          disabled={actionState.applyDisabled}
          title="应用"
        >
          {actionState.applyBusy ? <LoaderCircle className="spin" /> : <Check />}
        </button>
        <button
          type="button"
          className="icon-button icon-button-sm review-suggestion-row-action is-delete"
          onClick={onDelete}
          disabled={actionState.deleteDisabled}
          title="删除"
        >
          {actionState.deleteBusy ? <LoaderCircle className="spin" /> : <Trash2 />}
        </button>
        <button
          type="button"
          className="icon-button icon-button-sm review-suggestion-row-action"
          onClick={onToggleMenu}
          title="更多"
        >
          <MoreHorizontal />
        </button>
      </div>

      {menuOpen ? (
        <div className="review-suggestion-row-menu">
          <button
            type="button"
            className="review-suggestion-row-menu-item"
            onClick={onDismiss}
            disabled={actionState.dismissDisabled}
          >
            {actionState.dismissBusy ? <LoaderCircle className="spin" /> : <X />}
            <span>{secondaryLabel}</span>
          </button>
          {actionState.retryVisible ? (
            <button
              type="button"
              className="review-suggestion-row-menu-item"
              onClick={onRetry}
              disabled={actionState.retryDisabled}
            >
              {actionState.retryBusy ? <LoaderCircle className="spin" /> : <RotateCcw />}
              <span>重试</span>
            </button>
          ) : null}
        </div>
      ) : null}
    </div>
  );
});
```

- [ ] **Step 4: Rewrite `SuggestionReviewPane.tsx` as summary strip + row list and route existing actions into rows**

```tsx
import { memo, useMemo, useState } from "react";
import { AlertCircle, RotateCcw } from "lucide-react";
import type { DocumentSession, RewriteSuggestion, RewriteUnit } from "../../../lib/types";
import type { SessionStats } from "../../../lib/helpers";
import { countCharacters, getLatestSuggestion } from "../../../lib/helpers";
import { ReviewSuggestionRow } from "./ReviewSuggestionRow";
import { buildSuggestionRowActionState } from "./reviewSuggestionRowModel";

interface SuggestionReviewPaneProps {
  settingsReady: boolean;
  currentSession: DocumentSession;
  currentStats: SessionStats;
  activeRewriteUnit: RewriteUnit | null;
  activeSuggestionId: string | null;
  orderedSuggestions: RewriteSuggestion[];
  anyBusy: boolean;
  busyAction: string | null;
  rewriteRunning: boolean;
  rewritePaused: boolean;
  onSelectRewriteUnit: (rewriteUnitId: string, options?: { multiSelect?: boolean }) => void;
  onSelectSuggestion: (suggestionId: string) => void;
  onApplySuggestion: (suggestionId: string) => void;
  onDismissSuggestion: (suggestionId: string) => void;
  onDeleteSuggestion: (suggestionId: string) => void;
  onRetry: () => void;
}

export const SuggestionReviewPane = memo(function SuggestionReviewPane({
  settingsReady,
  currentSession,
  currentStats,
  activeRewriteUnit,
  activeSuggestionId,
  orderedSuggestions,
  anyBusy,
  busyAction,
  rewriteRunning,
  rewritePaused,
  onSelectRewriteUnit,
  onSelectSuggestion,
  onApplySuggestion,
  onDismissSuggestion,
  onDeleteSuggestion,
  onRetry
}: SuggestionReviewPaneProps) {
  const [openMenuSuggestionId, setOpenMenuSuggestionId] = useState<string | null>(null);
  const latestSuggestion = useMemo(() => getLatestSuggestion(currentSession), [currentSession]);
  const currentSequence =
    orderedSuggestions.find((item) => item.id === activeSuggestionId)?.sequence ??
    latestSuggestion?.sequence ??
    null;

  return (
    <>
      <div className="review-summary-strip">
        <span className="context-chip">修改对：{currentStats.suggestionsTotal}</span>
        <span className="context-chip">待审阅：{currentStats.suggestionsProposed}</span>
        <span className="context-chip">已应用：{currentStats.unitsApplied}/{currentStats.total}</span>
        {currentSequence ? <span className="context-chip">当前 #{currentSequence}</span> : null}
      </div>

      {activeRewriteUnit?.status === "failed" && !activeSuggestionId ? (
        <div className="error-card">
          <AlertCircle />
          <div>
            <strong>该片段生成失败</strong>
            <span>{activeRewriteUnit.errorMessage ?? "请点击重试重新生成。"}</span>
          </div>
          <button type="button" className="icon-button icon-button-sm" onClick={onRetry}>
            <RotateCcw />
          </button>
        </div>
      ) : null}

      <div className="suggestion-list scroll-region">
        {orderedSuggestions.map((suggestion) => (
          <ReviewSuggestionRow
            key={suggestion.id}
            suggestion={suggestion}
            active={suggestion.id === activeSuggestionId}
            menuOpen={openMenuSuggestionId === suggestion.id}
            actionState={buildSuggestionRowActionState({
              suggestionId: suggestion.id,
              decision: suggestion.decision,
              busyAction,
              anyBusy,
              editorMode: false,
              rewriteRunning,
              rewritePaused,
              settingsReady,
              rewriteUnitFailed:
                activeRewriteUnit?.id === suggestion.rewriteUnitId &&
                activeRewriteUnit.status === "failed"
            })}
            onSelect={() => {
              setOpenMenuSuggestionId(null);
              onSelectRewriteUnit(suggestion.rewriteUnitId);
              onSelectSuggestion(suggestion.id);
            }}
            onApply={() => onApplySuggestion(suggestion.id)}
            onDelete={() => onDeleteSuggestion(suggestion.id)}
            onDismiss={() => onDismissSuggestion(suggestion.id)}
            onRetry={onRetry}
            onToggleMenu={() =>
              setOpenMenuSuggestionId((current) =>
                current === suggestion.id ? null : suggestion.id
              )
            }
          />
        ))}
      </div>
    </>
  );
});
```

- [ ] **Step 5: Slim `ReviewActionBar.tsx` to a status-only header and route the new props through `ReviewPanel.tsx`**

```tsx
// ReviewActionBar.tsx
interface ReviewActionBarProps {
  editorMode: boolean;
  settingsReady: boolean;
  currentSession: DocumentSession | null;
  activeRewriteUnit: RewriteUnit | null;
  activeRewriteUnitSuggestions: RewriteSuggestion[];
  activeSuggestion: RewriteSuggestion | null;
}

export const ReviewActionBar = memo(function ReviewActionBar({
  editorMode,
  settingsReady,
  currentSession,
  activeRewriteUnit,
  activeRewriteUnitSuggestions,
  activeSuggestion
}: ReviewActionBarProps) {
  return (
    <div className={`workbench-action-reel ${editorMode ? "is-editor" : ""}`}>
      <div className="workbench-action-track">
        <div className="workbench-review-actionbar workbench-action-row is-normal" aria-hidden={editorMode}>
          <div className="workbench-review-actionbar-status">
            {activeSuggestion ? (
              <StatusBadge tone={suggestionTone(activeSuggestion.decision)}>
                #{activeSuggestion.sequence} {formatSuggestionDecision(activeSuggestion.decision)}
              </StatusBadge>
            ) : currentSession && activeRewriteUnit ? (
              <StatusBadge
                tone={rewriteUnitStatusTone(
                  currentSession,
                  activeRewriteUnit,
                  activeRewriteUnitSuggestions
                )}
              >
                {formatRewriteUnitStatus(
                  currentSession,
                  activeRewriteUnit,
                  activeRewriteUnitSuggestions
                )}
              </StatusBadge>
            ) : (
              <StatusBadge tone={settingsReady ? "info" : "warning"}>
                {settingsReady ? "等待生成" : "未配置"}
              </StatusBadge>
            )}
          </div>
        </div>
      </div>
    </div>
  );
});

// ReviewPanel.tsx
<SuggestionReviewPane
  settingsReady={settingsReady}
  currentSession={currentSession}
  currentStats={currentStats}
  activeRewriteUnit={activeRewriteUnit}
  activeSuggestionId={activeSuggestionId}
  orderedSuggestions={orderedSuggestions}
  anyBusy={anyBusy}
  busyAction={busyAction}
  rewriteRunning={rewriteRunning ?? false}
  rewritePaused={rewritePaused ?? false}
  onSelectRewriteUnit={onSelectRewriteUnit}
  onSelectSuggestion={onSelectSuggestion}
  onApplySuggestion={onApplySuggestion}
  onDismissSuggestion={onDismissSuggestion}
  onDeleteSuggestion={onDeleteSuggestion}
  onRetry={onRetry}
/>
```

- [ ] **Step 6: Run TypeScript validation for the rebuilt right rail**

Run: `pnpm run typecheck`

Expected: PASS with no prop/type errors in `SuggestionReviewPane`, `ReviewSuggestionRow`, `ReviewActionBar`, or `ReviewPanel`.

- [ ] **Step 7: Commit the component-tree refactor**

```bash
git add \
  src/stages/workbench/review/reviewSuggestionRowModel.ts \
  src/stages/workbench/review/ReviewSuggestionRow.tsx \
  src/stages/workbench/review/SuggestionReviewPane.tsx \
  src/stages/workbench/review/ReviewActionBar.tsx \
  src/stages/workbench/ReviewPanel.tsx \
  scripts/ui-regression.test.mjs
git commit -m "重构审阅右栏列表"
```

### Task 3: Delete the obsolete `reviewView` state chain

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/stages/WorkbenchStage.tsx`
- Modify: `src/stages/workbench/ReviewPanel.tsx`
- Modify: `src/app/hooks/useSuggestionActions.ts`
- Modify: `src/app/hooks/useRewriteActions.ts`
- Modify: `src/app/hooks/useDocumentActions.ts`
- Modify: `src/app/hooks/useDocumentFinalizeActions.ts`
- Modify: `src/lib/constants.ts`
- Modify: `scripts/ui-regression.test.mjs`

- [ ] **Step 1: Add failing regression assertions that the old `reviewView` chain is gone**

```js
const workbenchStage = read("src/stages/WorkbenchStage.tsx");
const reviewPanel = read("src/stages/workbench/ReviewPanel.tsx");
const suggestionActions = read("src/app/hooks/useSuggestionActions.ts");
const rewriteActions = read("src/app/hooks/useRewriteActions.ts");

assertNotIncludes(appSource, "const [reviewView, setReviewView]");
assertNotIncludes(appSource, 'import type { ReviewView } from "./lib/constants";');
assertNotIncludes(settingsConstants, "export type ReviewView");
assertNotIncludes(settingsConstants, "REVIEW_VIEW_OPTIONS");
assertNotIncludes(workbenchStage, "reviewView:");
assertNotIncludes(workbenchStage, "onSetReviewView:");
assertNotIncludes(reviewPanel, "reviewView:");
assertNotIncludes(reviewPanel, "onSetReviewView:");
assertNotIncludes(suggestionActions, "setReviewView");
assertNotIncludes(rewriteActions, "setReviewView");
```

- [ ] **Step 2: Run the regression script and verify it fails until the old state chain is removed**

Run: `node scripts/ui-regression.test.mjs`

Expected: FAIL because `App.tsx`, `WorkbenchStage.tsx`, `ReviewPanel.tsx`, and hooks still reference `reviewView`.

- [ ] **Step 3: Remove the state, props, and reset calls from `App.tsx`, stage props, and hooks**

```tsx
// App.tsx
import { DEFAULT_SETTINGS } from "./lib/constants";

export default function App() {
  const [activeRewriteUnitId, setActiveRewriteUnitId] = useState<string | null>(null);
  const [activeSuggestionId, setActiveSuggestionId] = useState<string | null>(null);
  const [selectedRewriteUnitIds, setSelectedRewriteUnitIds] = useState<string[]>([]);

  useTauriEvents({
    onRewriteUnitCompleted: async (payload) => {
      const session = currentSessionRef.current;
      if (session && payload.sessionId === session.id) {
        await refreshSessionState(payload.sessionId, {
          preferredRewriteUnitId: payload.rewriteUnitId,
          preferredSuggestionId: payload.suggestionId
        });
      }
    }
  });

  <WorkbenchStage
    settings={settings}
    currentSession={currentSession}
    liveProgress={liveProgress}
    currentStats={currentStats}
    activeRewriteUnit={activeRewriteUnit}
    activeRewriteUnitId={activeRewriteUnitId}
    activeSuggestionId={activeSuggestionId}
    selectedRewriteUnitIds={selectedRewriteUnitIds}
    busyAction={busyAction}
    editorMode={stage === "editor"}
    editorText={editorText}
    editorSlotOverrides={editorSlotOverrides}
    editorDirty={editorDirty}
    editorHasSelection={editorHasSelection}
    editorRef={editorRef}
    documentScrollRef={documentScrollRef}
    onOpenDocument={handleOpenDocument}
    onSelectRewriteUnit={suggestionActions.handleSelectRewriteUnit}
    onSelectSuggestion={suggestionActions.handleSelectSuggestion}
    onStartRewrite={rewriteActions.handleStartRewrite}
    // no reviewView / onSetReviewView props
  />
```

```ts
// useSuggestionActions.ts
export function useSuggestionActions(options: {
  currentSessionRef: React.MutableRefObject<DocumentSession | null>;
  activeRewriteUnitIdRef: React.MutableRefObject<string | null>;
  captureDocumentScrollPosition: () => number | null;
  setActiveRewriteUnitId: React.Dispatch<React.SetStateAction<string | null>>;
  setActiveSuggestionId: React.Dispatch<React.SetStateAction<string | null>>;
  setSelectedRewriteUnitIds: React.Dispatch<React.SetStateAction<string[]>>;
  applySessionState: ApplySessionState;
  refreshSessionState: RefreshSessionState;
  showNotice: ShowNotice;
  withBusy: WithBusy;
}) { /* ... */ }

const handleSelectSuggestion = useCallback(
  (suggestionId: string) => {
    setActiveSuggestionId(suggestionId);
  },
  [setActiveSuggestionId]
);
```

```ts
// src/lib/constants.ts
import type { AppSettings, SegmentationPreset, RewriteMode } from "./types";

export const DEFAULT_SETTINGS: AppSettings = {
  baseUrl: "https://api.openai.com/v1",
  apiKey: "",
  model: "gpt-4.1-mini",
  updateProxy: "",
  timeoutMs: 45_000,
  temperature: 0.8,
  segmentationPreset: "paragraph",
  rewriteHeadings: false,
  rewriteMode: "manual",
  maxConcurrency: 2,
  unitsPerBatch: 1,
  promptPresetId: "humanizer_zh",
  customPrompts: []
};
```

- [ ] **Step 4: Run TypeScript validation after deleting the old state chain**

Run: `pnpm run typecheck`

Expected: PASS with no remaining `ReviewView`, `reviewView`, or `setReviewView` references.

- [ ] **Step 5: Commit the state cleanup**

```bash
git add \
  src/App.tsx \
  src/stages/WorkbenchStage.tsx \
  src/stages/workbench/ReviewPanel.tsx \
  src/app/hooks/useSuggestionActions.ts \
  src/app/hooks/useRewriteActions.ts \
  src/app/hooks/useDocumentActions.ts \
  src/app/hooks/useDocumentFinalizeActions.ts \
  src/lib/constants.ts \
  scripts/ui-regression.test.mjs
git commit -m "清理审阅视图旧状态"
```

### Task 4: Replace the old right-rail styles and lock the new layout with regression checks

**Files:**
- Modify: `src/styles/part-02.css`
- Modify: `src/styles/part-04.css`
- Modify: `scripts/ui-regression.test.mjs`
- Test: `src/stages/workbench/review/ReviewSuggestionRow.tsx`
- Test: `src/stages/workbench/review/SuggestionReviewPane.tsx`

- [ ] **Step 1: Add failing CSS and markup assertions for the compact list layout**

```js
const reviewSuggestionRow = read("src/stages/workbench/review/ReviewSuggestionRow.tsx");

assertIncludes(reviewSuggestionRow, 'className="review-suggestion-row-main"');
assertIncludes(reviewSuggestionRow, 'className="review-suggestion-row-actions"');
assertIncludes(reviewSuggestionRow, 'className="review-suggestion-row-menu"');
assertIncludes(part04, ".review-summary-strip");
assertIncludes(part04, ".review-suggestion-row-actions");
assertIncludes(part04, ".review-suggestion-row-menu");
assertRule(part04, ".review-suggestion-row-actions", "opacity", "0");
assertRule(part04, ".review-suggestion-row:hover .review-suggestion-row-actions", "opacity", "1");
assertNotIncludes(part04, ".review-switches");
assertNoRule(part02, ".workbench-review-body .diff-view", "max-height", "260px");
```

- [ ] **Step 2: Run the regression script and verify it fails until the new classes and CSS exist**

Run: `node scripts/ui-regression.test.mjs`

Expected: FAIL because the new `.review-summary-strip` / `.review-suggestion-row-*` rules do not exist yet.

- [ ] **Step 3: Replace the old review-body rules in `part-02.css` and add compact right-rail styles in `part-04.css`**

```css
/* part-02.css */
.panel-body.workbench-review-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  overflow: hidden;
}

.workbench-review-body .suggestion-list {
  flex: 1;
  min-height: 0;
  overflow: auto;
  padding-bottom: 2px;
}

/* part-04.css */
.review-summary-strip {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
  align-items: center;
}

.review-suggestion-row {
  position: relative;
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
  align-items: center;
  padding: 8px 10px;
  border: var(--border);
  border-radius: var(--radius-md);
  background: rgba(255, 255, 255, 0.76);
}

.review-suggestion-row.is-active {
  background: linear-gradient(135deg, rgba(23, 68, 207, 0.12), rgba(255, 250, 241, 0.92));
}

.review-suggestion-row-main {
  min-width: 0;
  display: grid;
  gap: 4px;
  text-align: left;
}

.review-suggestion-row-mainline {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.review-suggestion-row-title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-weight: 650;
}

.review-suggestion-row-actions {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--transition), transform var(--transition);
}

.review-suggestion-row:hover .review-suggestion-row-actions,
.review-suggestion-row.is-active .review-suggestion-row-actions {
  opacity: 1;
  pointer-events: auto;
}

.review-suggestion-row-menu {
  position: absolute;
  top: calc(100% - 2px);
  right: 10px;
  z-index: 3;
  min-width: 132px;
  padding: 6px;
  border: var(--border);
  border-radius: 14px;
  background: rgba(255, 255, 255, 0.98);
  box-shadow: 0 12px 28px rgba(20, 20, 20, 0.16);
}
```

- [ ] **Step 4: Remove the obsolete review-detail selectors that no longer have any callers**

```css
/* delete from part-04.css */
.review-switches { /* ... */ }
.review-switches .switch-chip { /* ... */ }
.diff-view { /* ... */ }
.diff-view p { /* ... */ }

/* delete from part-02.css */
.workbench-review-body .diff-view,
.workbench-review-body .preview-text,
.workbench-review-body .text-scroll {
  flex: 0 0 auto;
  max-height: 260px;
}
```

- [ ] **Step 5: Re-run the full static validation**

Run: `node scripts/ui-regression.test.mjs`

Expected: PASS with the new right-rail assertions and no stale `reviewView` / `diff-view` references.

- [ ] **Step 6: Re-run TypeScript validation**

Run: `pnpm run typecheck`

Expected: PASS with no CSS-driven markup mismatches or component import errors.

- [ ] **Step 7: Commit the final layout pass**

```bash
git add \
  src/styles/part-02.css \
  src/styles/part-04.css \
  src/stages/workbench/review/ReviewSuggestionRow.tsx \
  src/stages/workbench/review/SuggestionReviewPane.tsx \
  src/stages/workbench/review/ReviewActionBar.tsx \
  src/stages/workbench/ReviewPanel.tsx \
  scripts/ui-regression.test.mjs
git commit -m "实现审阅右栏紧凑交互"
```

### Task 5: Smoke-test the workbench interaction end-to-end

**Files:**
- Test: `src/stages/workbench/review/SuggestionReviewPane.tsx`
- Test: `src/stages/workbench/review/ReviewSuggestionRow.tsx`
- Test: `src/stages/workbench/document/ParagraphDocumentFlow.tsx`

- [ ] **Step 1: Launch the desktop app**

Run: `pnpm run tauri:dev`

Expected: Vite + Tauri start successfully and the workbench opens without runtime errors.

- [ ] **Step 2: Verify row selection still drives left-document positioning**

Manual check:

1. Open a document with at least three suggestions.
2. Click suggestion row `#N` in the right rail.
3. Confirm the left document pane scrolls to the corresponding rewrite unit and keeps that unit highlighted.

Expected: the right rail does not need its own diff viewer to understand the current suggestion.

- [ ] **Step 3: Verify hover actions and secondary menu behavior**

Manual check:

1. Hover a non-active row.
2. Confirm only that row reveals `应用` / `删除` / `···`.
3. Open `···` and confirm the secondary label is `忽略`.
4. Apply one row, reopen `···`, and confirm the secondary label becomes `取消应用`.

Expected: hover actions are scoped to one row and the secondary label matches suggestion state.

- [ ] **Step 4: Verify the failed-unit edge case remains operable**

Manual check:

1. Force or load a session where the active rewrite unit is `failed`.
2. Confirm the right rail still exposes a retry path even when no detail viewer is shown.
3. Trigger retry once and confirm the session refreshes without runtime errors.

Expected: failed units are not orphaned by removal of the old global action buttons.

- [ ] **Step 5: Review the final diff before handoff**

Run: `git diff -- src/App.tsx src/stages/WorkbenchStage.tsx src/stages/workbench/ReviewPanel.tsx src/stages/workbench/review src/styles/part-02.css src/styles/part-04.css scripts/ui-regression.test.mjs`

Expected: only the compact right-rail implementation, `reviewView` cleanup, and corresponding regression coverage are present.
