export const SCROLL_RESTORE_TOLERANCE_PX = 1;
export const REQUIRED_SCROLL_STABLE_FRAMES = 2;
export const MAX_SCROLL_RESTORE_ATTEMPTS = 8;

export interface ScrollRestoreProgress {
  targetScrollTop: number;
  attempts: number;
  stableFrames: number;
}

export function beginScrollRestore(targetScrollTop: number): ScrollRestoreProgress {
  return {
    targetScrollTop,
    attempts: 0,
    stableFrames: 0
  };
}

export function advanceScrollRestore(
  progress: ScrollRestoreProgress,
  actualScrollTop: number
) {
  const isStable =
    Math.abs(actualScrollTop - progress.targetScrollTop) <= SCROLL_RESTORE_TOLERANCE_PX;
  const next = {
    targetScrollTop: progress.targetScrollTop,
    attempts: progress.attempts + 1,
    stableFrames: isStable ? progress.stableFrames + 1 : 0
  };

  return {
    next,
    done:
      next.stableFrames >= REQUIRED_SCROLL_STABLE_FRAMES ||
      next.attempts >= MAX_SCROLL_RESTORE_ATTEMPTS
  } as const;
}
