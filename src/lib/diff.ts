import type { DiffSpan } from "./types";
import { normalizeNewlines } from "./helpers";

type DiffOp =
  | { type: "equal"; value: string }
  | { type: "insert"; value: string }
  | { type: "delete"; value: string };

const MAX_REFINED_CHARS = 8_000;

export interface DiffHunk {
  id: string;
  sequence: number;
  diffSpans: DiffSpan[];
  beforeText: string;
  afterText: string;
  insertedChars: number;
  deletedChars: number;
}

function splitLinesPreserveNewline(text: string): string[] {
  if (text.length === 0) return [""];
  const lines = text.split("\n");
  return lines.map((line, index) => (index < lines.length - 1 ? `${line}\n` : line));
}

function splitChars(text: string): string[] {
  return Array.from(text);
}

function myersDiff(before: ReadonlyArray<string>, after: ReadonlyArray<string>) {
  const n = before.length;
  const m = after.length;
  const max = n + m;
  const offset = max;
  const v = new Int32Array(2 * max + 1);

  // 使用 -1 作为哨兵值，便于比较。
  v.fill(-1);
  v[offset + 1] = 0;

  const trace: Int32Array[] = [];
  let finished = false;

  for (let d = 0; d <= max; d++) {
    for (let k = -d; k <= d; k += 2) {
      const kIndex = offset + k;

      let x: number;
      if (
        k === -d ||
        (k !== d && v[offset + k - 1] < v[offset + k + 1])
      ) {
        // 向下走：插入 after[y]
        x = v[offset + k + 1];
      } else {
        // 向右走：删除 before[x]
        x = v[offset + k - 1] + 1;
      }

      let y = x - k;
      while (x < n && y < m && before[x] === after[y]) {
        x += 1;
        y += 1;
      }

      v[kIndex] = x;

      if (x >= n && y >= m) {
        finished = true;
        break;
      }
    }

    trace.push(new Int32Array(v));
    if (finished) break;
  }

  // 回溯生成操作序列（forward order）
  let x = n;
  let y = m;
  const ops: DiffOp[] = [];

  for (let d = trace.length - 1; d > 0; d--) {
    const vPrev = trace[d - 1];
    const k = x - y;

    let prevK: number;
    if (k === -d || (k !== d && vPrev[offset + k - 1] < vPrev[offset + k + 1])) {
      prevK = k + 1;
    } else {
      prevK = k - 1;
    }

    const prevX = vPrev[offset + prevK];
    const prevY = prevX - prevK;

    while (x > prevX && y > prevY) {
      ops.push({ type: "equal", value: before[x - 1] });
      x -= 1;
      y -= 1;
    }

    if (x === prevX) {
      ops.push({ type: "insert", value: after[prevY] });
      y -= 1;
    } else {
      ops.push({ type: "delete", value: before[prevX] });
      x -= 1;
    }

    x = prevX;
    y = prevY;
  }

  while (x > 0 && y > 0) {
    ops.push({ type: "equal", value: before[x - 1] });
    x -= 1;
    y -= 1;
  }

  while (x > 0) {
    ops.push({ type: "delete", value: before[x - 1] });
    x -= 1;
  }

  while (y > 0) {
    ops.push({ type: "insert", value: after[y - 1] });
    y -= 1;
  }

  return ops.reverse();
}

function pushSpan(spans: DiffSpan[], type: DiffSpan["type"], text: string) {
  if (!text) return;
  const last = spans[spans.length - 1];
  if (last && last.type === type) {
    last.text += text;
    return;
  }
  spans.push({ type, text });
}

export function diffTextByChars(beforeText: string, afterText: string): DiffSpan[] {
  const ops = myersDiff(splitChars(beforeText), splitChars(afterText));
  const spans: DiffSpan[] = [];

  for (const op of ops) {
    const type: DiffSpan["type"] =
      op.type === "equal" ? "unchanged" : op.type === "insert" ? "insert" : "delete";
    pushSpan(spans, type, op.value);
  }

  return spans;
}

export function diffTextByLines(beforeText: string, afterText: string): DiffSpan[] {
  const normalizedBefore = normalizeNewlines(beforeText);
  const normalizedAfter = normalizeNewlines(afterText);

  if (normalizedBefore === normalizedAfter) {
    return [{ type: "unchanged", text: normalizedAfter }];
  }

  const before = splitLinesPreserveNewline(normalizedBefore);
  const after = splitLinesPreserveNewline(normalizedAfter);
  const ops = myersDiff(before, after);

  const spans: DiffSpan[] = [];
  let pendingDeletes: string[] = [];
  let pendingInserts: string[] = [];

  const flushPending = () => {
    if (pendingDeletes.length === 0 && pendingInserts.length === 0) return;

    const deletedText = pendingDeletes.join("");
    const insertedText = pendingInserts.join("");
    pendingDeletes = [];
    pendingInserts = [];

    if (deletedText && insertedText) {
      const refined =
        deletedText.length + insertedText.length <= MAX_REFINED_CHARS
          ? diffTextByChars(deletedText, insertedText)
          : null;

      if (refined) {
        for (const span of refined) {
          pushSpan(spans, span.type, span.text);
        }
        return;
      }

      pushSpan(spans, "delete", deletedText);
      pushSpan(spans, "insert", insertedText);
      return;
    }

    if (deletedText) pushSpan(spans, "delete", deletedText);
    if (insertedText) pushSpan(spans, "insert", insertedText);
  };

  for (const op of ops) {
    if (op.type === "equal") {
      flushPending();
      pushSpan(spans, "unchanged", op.value);
      continue;
    }
    if (op.type === "delete") {
      pendingDeletes.push(op.value);
      continue;
    }
    pendingInserts.push(op.value);
  }

  flushPending();
  return spans;
}
