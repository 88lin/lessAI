import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { TAURI_EVENTS } from "../lib/constants";
import type {
  ChunkCompletedPayload,
  RewriteFailedPayload,
  SessionEventPayload
} from "../lib/constants";
import type { RewriteProgress } from "../lib/types";

interface TauriEventHandlers {
  onProgress: (payload: RewriteProgress) => void;
  onChunkCompleted: (payload: ChunkCompletedPayload) => void;
  onFinished: (payload: SessionEventPayload) => void;
  onFailed: (payload: RewriteFailedPayload) => void;
}

/**
 * 注册 4 个 Tauri 事件监听器，使用 Promise.all 确保原子性注册。
 * 通过 ref 持有最新回调，避免闭包捕获旧值。
 */
export function useTauriEvents(handlers: TauriEventHandlers) {
  const handlersRef = useRef(handlers);
  handlersRef.current = handlers;

  useEffect(() => {
    let mounted = true;
    let cleanup: (() => void) | null = null;

    void (async () => {
      const unlisteners = await Promise.all([
        listen<RewriteProgress>(TAURI_EVENTS.REWRITE_PROGRESS, ({ payload }) => {
          handlersRef.current.onProgress(payload);
        }),
        listen<ChunkCompletedPayload>(TAURI_EVENTS.CHUNK_COMPLETED, ({ payload }) => {
          handlersRef.current.onChunkCompleted(payload);
        }),
        listen<SessionEventPayload>(TAURI_EVENTS.REWRITE_FINISHED, ({ payload }) => {
          handlersRef.current.onFinished(payload);
        }),
        listen<RewriteFailedPayload>(TAURI_EVENTS.REWRITE_FAILED, ({ payload }) => {
          handlersRef.current.onFailed(payload);
        })
      ]);

      if (!mounted) {
        for (const unlisten of unlisteners) {
          void unlisten();
        }
        return;
      }

      cleanup = () => {
        for (const unlisten of unlisteners) {
          void unlisten();
        }
      };
    })();

    return () => {
      mounted = false;
      cleanup?.();
    };
  }, []);
}
