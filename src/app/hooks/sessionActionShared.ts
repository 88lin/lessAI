import type { NoticeTone } from "../../lib/constants";
import type { DocumentSession } from "../../lib/types";
import { canRewriteSession, rewriteBlockedReason } from "../../lib/helpers";

export type ShowNotice = (
  tone: NoticeTone,
  message: string,
  options?: { autoDismissMs?: number | null }
) => void;

export type WithBusy = <T>(action: string, fn: () => Promise<T>) => Promise<T>;

export type ApplySessionState = (
  session: DocumentSession,
  nextChunkIndex: number,
  options?: { preferredSuggestionId?: string | null }
) => void;

export interface RefreshSessionOptions {
  preserveChunk?: boolean;
  preferredChunkIndex?: number;
  preserveSuggestion?: boolean;
  preferredSuggestionId?: string | null;
}

export type RefreshSessionState = (
  sessionId: string,
  options?: RefreshSessionOptions
) => Promise<DocumentSession>;

interface RefreshSessionOrNotifyOptions {
  session: DocumentSession;
  refreshSessionState: RefreshSessionState;
  options?: RefreshSessionOptions;
  showNotice: ShowNotice;
  errorPrefix: string;
  formatError: (error: unknown) => string;
}

export async function refreshSessionOrNotify({
  session,
  refreshSessionState,
  options,
  showNotice,
  errorPrefix,
  formatError
}: RefreshSessionOrNotifyOptions): Promise<DocumentSession | null> {
  try {
    return await refreshSessionState(session.id, options);
  } catch (error) {
    showNotice("error", `${errorPrefix}：${formatError(error)}`);
    return null;
  }
}

interface RefreshAllowedSessionOrNotifyOptions extends RefreshSessionOrNotifyOptions {
  allowed: (session: DocumentSession) => boolean;
  blockedMessage: (session: DocumentSession) => string | null | undefined;
  fallbackMessage: string;
}

export async function refreshAllowedSessionOrNotify({
  session,
  refreshSessionState,
  options,
  showNotice,
  errorPrefix,
  formatError,
  allowed,
  blockedMessage,
  fallbackMessage
}: RefreshAllowedSessionOrNotifyOptions): Promise<DocumentSession | null> {
  const latestSession = await refreshSessionOrNotify({
    session,
    refreshSessionState,
    options,
    showNotice,
    errorPrefix,
    formatError
  });
  if (!latestSession) {
    return null;
  }
  if (
    !ensureAllowedOrNotify({
      allowed: allowed(latestSession),
      blockedMessage: blockedMessage(latestSession),
      fallbackMessage,
      showNotice
    })
  ) {
    return null;
  }
  return latestSession;
}

interface RefreshRewriteableSessionOrNotifyOptions extends RefreshSessionOrNotifyOptions {}

export async function refreshRewriteableSessionOrNotify({
  session,
  refreshSessionState,
  options,
  showNotice,
  errorPrefix,
  formatError
}: RefreshRewriteableSessionOrNotifyOptions): Promise<DocumentSession | null> {
  return refreshAllowedSessionOrNotify({
    session,
    refreshSessionState,
    options,
    showNotice,
    errorPrefix,
    formatError,
    allowed: canRewriteSession,
    blockedMessage: rewriteBlockedReason,
    fallbackMessage: "当前文档暂不支持安全写回覆盖，因此不允许继续 AI 改写。"
  });
}

interface EnsureAllowedOrNotifyOptions {
  allowed: boolean;
  blockedMessage: string | null | undefined;
  fallbackMessage: string;
  showNotice: ShowNotice;
}

export function ensureAllowedOrNotify({
  allowed,
  blockedMessage,
  fallbackMessage,
  showNotice
}: EnsureAllowedOrNotifyOptions): boolean {
  if (allowed) return true;
  showNotice("warning", blockedMessage ?? fallbackMessage);
  return false;
}
